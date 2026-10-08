// ============================================================
// src/tests/bridge_pipeline_tests.rs
//
// V10: Afsi'nin istediği 5 adımlık zinciri, GERÇEK bridge
// fonksiyonları üzerinden (aynı kod, Flutter'ın çağıracağı fonksiyonlar)
// terminalde doğrular — Flutter codegen'i beklemeden.
//
//   1. compile_wat_to_wasm  → hash
//   2. register_agent_script(name, hash, entrypoint, timeout_ms)
//   3. start_agent(objective, ...) → agent_id, execution_id
//   4. grant_agent_capability(agent_id, "wasm_execution")
//   5. get_agent_status(execution_id) ile poll et
//
// ÖNEMLİ KISITLAMA (dürüstçe test edilen budur):
// Bu test ortamında hiçbir AI provider yapılandırılı değil, bu yüzden
// start_agent'ın planlaması fallback_plan'a düşer — ki o (Sprint 2'de
// kanıtlandığı gibi) HİÇBİR adıma tool_call eklemez. Yani bu test,
// script kaydının, capability grant'in ve start/status akışının
// GERÇEKTEN çalıştığını kanıtlar; ama "AI script'i seçip çağırdı"
// senaryosu gerçek bir provider gerektirir ve burada test edilmiyor.
// Onu ayrıca, aynı gerçek script_registry/script_engine/capability_engine
// üzerinden invoke_tool_call ile doğrudan tetikleyerek kapatıyoruz —
// "grant edilince gerçekten çalışırdı" iddiasını AI'ye bağımlı olmadan
// kanıtlamak için.
// ============================================================

use std::time::Duration;

use crate::agents::plans::ToolCall;

#[test]
fn bridge_pipeline_register_grant_and_run() {
    // ── Hazırlık: her test çalıştırmasında temiz, izole bir runtime ──
    let db_path = std::env::temp_dir()
        .join(format!("aetheros_test_{}", uuid::Uuid::new_v4()))
        .join("aetheros.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();

    // init_mobile_runtime artık kendi ortamını kendi sağlıyor
    // (bkz. bridge/state.rs'teki tokio.enter() düzeltmesi) — bu
    // yüzden burada özel bir ortam kurmamıza gerek yok, düz bir
    // çağrı yeterli.
    crate::bridge::state::init_mobile_runtime(db_path.display().to_string(), 2)
        .expect("runtime başlatılabilmeli");

    // Async adımları sürmek için KENDİ runtime'ımızı kuruyoruz —
    // init_mobile_runtime'ınkiyle çakışmaz, çünkü o zaten döndü.
    let driver = tokio::runtime::Runtime::new().unwrap();

    driver.block_on(async {
        // ── Adım 1: WASM yükle (compile_wat_to_wasm) ──────────
        let module = crate::bridge::api::compile_wat_to_wasm(
            "noop_script".to_string(),
            r#"(module (func (export "main")))"#.to_string(),
            "main".to_string(),
            1_000,
        )
        .await
        .expect("minimal WAT derlenebilmeli");

        assert!(!module.hash.is_empty());

        // ── Adım 2: script'i agent tool'u olarak kaydet ───────
        crate::bridge::api::register_agent_script(
            "noop_script".to_string(),
            module.hash.clone(),
            "main".to_string(),
            1_000,
        )
        .expect("script kaydedilebilmeli");

        let scripts = crate::bridge::api::list_agent_scripts().expect("liste alınabilmeli");
        assert!(
            scripts.contains(&"noop_script".to_string()),
            "kaydedilen script listede görünmeli"
        );

        // ── Adım 3: agent'ı başlat ─────────────────────────────
        let start = crate::bridge::api::start_agent("deneme".to_string(), 5, 1_000, vec![])
            .await
            .expect("agent başlatılabilmeli");

        // ── Adım 4: capability'yi HEMEN grant et ──────────────
        // start_agent döner dönmez, arada başka bir await olmadan —
        // bu, gerçek uygulamadaki yarış durumunun aynısı. Test
        // ortamında (fallback_plan çok hızlı tamamlandığından) bile
        // bu sıralama önemli; gerçek cihazda daha da kritik.
        crate::bridge::api::grant_agent_capability(
            start.agent_id.clone(),
            "wasm_execution".to_string(),
        )
        .expect("capability grant edilebilmeli");

        // ── Adım 5: durumu poll et ─────────────────────────────
        let mut final_status = None;
        for _ in 0..100 {
            let status = crate::bridge::api::get_agent_status(start.execution_id.clone())
                .expect("agent kaydı bulunabilmeli");

            if status.status != "running" {
                final_status = Some(status);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let status = final_status.expect("agent zaman aşımı olmadan sonuçlanmalı");
        // AI sağlayıcı yokken agent hiçbir şey planlayamaz: eskiden boş plan
        // "completed" diyordu (yanıltıcı). Artık dürüstçe "failed" + açık sebep.
        assert_eq!(
            status.status, "failed",
            "AI provider yokken agent başarısız sayılmalı: {:?}",
            status.error
        );
        assert!(
            status
                .error
                .as_deref()
                .is_some_and(|e| e.contains("AI sağlayıcı aktif değil")),
            "hata mesajı sebebi söylemeli: {:?}",
            status.error
        );

        // ── Zinciri kapatma: AI olmadan da "grant edilseydi
        // gerçekten çalışırdı" iddiasını doğrudan kanıtla ─────
        // fallback_plan hiçbir tool_call üretmediği için yukarıdaki
        // execution script'i fiilen çağırmadı (beklenen). Ama
        // register_agent_script + grant_agent_capability'nin GERÇEKTEN
        // kurduğu aynı script_registry/script_engine/capability_engine
        // üzerinden, yapısal bir ToolCall'ı elle tetikleyip agent'ın
        // gerçekten çalışabileceğini kanıtlıyoruz.
        let rt = crate::bridge::state::get_runtime().expect("runtime hâlâ ayakta olmalı");

        let tools: Vec<std::sync::Arc<dyn crate::types::agent_tool::AgentTool>> = rt
            .script_registry
            .list()
            .into_iter()
            .map(|def| {
                std::sync::Arc::new(crate::scripting::tool::ScriptTool::new(
                    def,
                    rt.script_engine.clone(),
                )) as std::sync::Arc<dyn crate::types::agent_tool::AgentTool>
            })
            .collect();

        let mut runtime = crate::agents::runtime::AgentRuntime::new(
            crate::agents::budget::AgentExecutionBudget {
                max_tokens: 1_000,
                max_steps: 5,
                max_runtime_seconds: 30,
            },
            tools,
            None,
            Some(rt.capability_engine.clone()),
            Some(rt.risk_engine.clone()),
            None,
            None,
        );
        // Bu test capability grant zincirini kanıtlıyor, Risk Engine'in
        // KENDİ blocking davranışını değil (o security/risk_engine.rs'in
        // testlerinde ayrıca kanıtlanıyor). ScriptTool High risk bildirdiği
        // ve varsayılan politika Block olduğu için, capability grant tek
        // başına artık yetmiyor — bunu burada bilerek gevşetiyoruz.
        runtime.set_high_risk_policy(crate::agents::runtime::HighRiskPolicy::AllowWithWarning);

        let agent_id = uuid::Uuid::parse_str(&start.agent_id).unwrap();
        let call = ToolCall {
            tool_name: "noop_script".to_string(),
            arguments: vec![],
        };

        let result = runtime.invoke_tool_call(agent_id, &call).await;
        assert!(
            result.is_ok(),
            "grant edilmiş agent, gerçekten kaydedilmiş script'i çalıştırabilmeli: {:?}",
            result.err()
        );

        // ── B11: yetkiler start_agent'ta, spawn'dan ÖNCE verilir ──
        use crate::agents::capabilities::AgentCapability as Cap;
        use crate::security::capability_engine::CapabilityDecision as Dec;

        // Bilinmeyen capability → Err, hiçbir agent kaydı/yan etki yok.
        let before = rt.agent_registry.len();
        let bad = crate::bridge::api::start_agent(
            "kötü".to_string(),
            5,
            1_000,
            vec!["wasm_execution".to_string(), "uçan_halı".to_string()],
        )
        .await;
        assert!(bad.is_err(), "bilinmeyen capability reddedilmeli");
        assert_eq!(
            rt.agent_registry.len(),
            before,
            "reddedilen start_agent hiçbir agent kaydı bırakmamalı"
        );

        // Geçerli yetkiler: dönüşte (arada hiçbir grant çağrısı olmadan)
        // zaten verilmiş olmalı; verilmeyenler reddedilmeli.
        let granted = crate::bridge::api::start_agent(
            "yetkili".to_string(),
            5,
            1_000,
            vec!["wasm_execution".to_string(), "wasm_execution".to_string()],
        )
        .await
        .expect("geçerli yetkilerle başlatılabilmeli");
        let gid = uuid::Uuid::parse_str(&granted.agent_id).unwrap();
        assert!(matches!(
            rt.capability_engine.check(&gid, Some(Cap::WasmExecution)),
            Dec::Allowed
        ));
        assert!(matches!(
            rt.capability_engine
                .check(&gid, Some(Cap::TerminalExecution)),
            Dec::Denied { .. }
        ));

        // Yetkisiz başlatılan agent hiçbir şeye sahip değil.
        let none = crate::bridge::api::start_agent("yetkisiz".to_string(), 5, 1_000, vec![])
            .await
            .unwrap();
        let nid = uuid::Uuid::parse_str(&none.agent_id).unwrap();
        assert!(matches!(
            rt.capability_engine.check(&nid, Some(Cap::WasmExecution)),
            Dec::Denied { .. }
        ));

        // Workspace yetkileri: ad → enum eşlemesi ve bağımsızlık.
        let wsr = crate::bridge::api::start_agent(
            "workspace okuyucu".to_string(),
            5,
            1_000,
            vec!["workspace_read".to_string()],
        )
        .await
        .expect("workspace_read geçerli bir yetki adı olmalı");
        let wid = uuid::Uuid::parse_str(&wsr.agent_id).unwrap();
        assert!(matches!(
            rt.capability_engine.check(&wid, Some(Cap::WorkspaceRead)),
            Dec::Allowed
        ));
        assert!(matches!(
            rt.capability_engine.check(&wid, Some(Cap::WorkspaceWrite)),
            Dec::Denied { .. }
        ));
        assert!(matches!(
            rt.capability_engine.check(&wid, Some(Cap::WorkspaceMutate)),
            Dec::Denied { .. }
        ));
        for name in ["workspace_write", "workspace_mutate"] {
            assert!(crate::bridge::api::parse_capability(name).is_ok(), "{name}");
        }

        // Workspace init'te {workspace_dir} üzerinde açılmış olmalı ve
        // agent araç listesine tüm workspace araçları eklenmeli.
        let root =
            crate::bridge::state::workspace_root().expect("workspace init'te açılmış olmalı");
        assert!(!root.is_empty());
        let mut tools: Vec<std::sync::Arc<dyn crate::types::agent_tool::AgentTool>> = Vec::new();
        crate::bridge::api::append_workspace_tools(rt, &mut tools);
        assert_eq!(
            tools.len(),
            crate::agents::workspace_tool::WorkspaceToolKind::ALL.len()
        );

        // Dosyalar ekranı köprüsü: aynı workspace üzerinde uçtan uca.
        assert_eq!(crate::bridge::api::workspace_files_root().unwrap(), root);
        crate::bridge::api::workspace_create_entry("ui_deneme.txt".to_string(), false).unwrap();
        let items = crate::bridge::api::workspace_list_dir(String::new(), false).unwrap();
        assert!(items.iter().any(|i| i.name == "ui_deneme.txt" && !i.is_dir));
        crate::bridge::api::workspace_delete_entry("ui_deneme.txt".to_string()).unwrap();
        assert!(crate::bridge::api::workspace_delete_entry(".".to_string()).is_err());
    });
}
