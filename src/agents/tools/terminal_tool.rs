// ============================================================
// src/agents/terminal_tool.rs
//
// V10 Faz 1: aetheros-terminal crate'ini, mevcut guarded AgentTool
// hattına (Governor → Approval → Audit) bağlayan adaptör.
//
// TASARIM İLKESİ ("elmas"): terminal, sistemin şimdiye dek gördüğü en
// tehlikeli yetenek — bu yüzden ne capability ne risk kararı burada
// ATLANMIYOR. Adaptör sadece kendini dürüstçe tanıtır:
//   - required_capability() → TerminalExecution (grant edilmeden çalışmaz)
//   - risk_level()          → High (varsayılan politika Block; onay
//                             akışı için HighRiskPolicy::RequireApproval)
// Asıl kararı yine SecurityGovernor verir; bu dosya karar VERMEZ.
//
// ARGÜMAN SÖZLEŞMESİ (shell YOK): arguments[0] = program,
// arguments[1..] = o programın argümanları. Örn. ["git", "status"].
// Bir shell string'i ("ls && rm -rf /") ASLA yorumlanmaz — `&&`, `;`,
// `$()` gibi meta-karakterler düz argüman olarak programa iletilir.
// Bu, CommandSpec'in tasarımından gelen gerçek bir enjeksiyon savunması.
//
// BİLİNEN SINIRLAMALAR (dürüstçe):
//   1. (B9 ile KISMEN ÇÖZÜLDÜ) Workspace verilmişse komut orada koşar ve
//      yol argümanları workspace dışına çıkamaz (security::path_confinement;
//      `/etc/passwd`, `../x`, dışarı işaret eden symlink → Deny). Program
//      kendi başına dışarıyı açarsa (yapılandırma, ağ) engellenemez; tam
//      hapis için sistem düzeyi sandbox gerekir.
//   2. (B4 ile ÇÖZÜLDÜ) Karar artık argüman-bazlı: assess_call →
//      security::command_policy (Deny > Allow > Ask). `sh -c` reddedilir,
//      `git status` onaysız çalışır, `git commit` onay ister. Yol-bazlı
//      kurallar (gizli dosya okuma) hâlâ yok — workspace kök hapsi işi.
//   3. Sadece tek-seferlik komut çalıştırma (one-shot). PTY/interaktif
//      oturumlar aetheros-terminal'de var ama bu adaptör onları
//      bilerek açmıyor.
// ============================================================

use std::path::PathBuf;
use std::sync::Arc;

use aetheros_terminal::{
    AgentTerminalTool, CommandBuilder, Environment, TerminalToolRequest, TerminalToolResponse,
};
use async_trait::async_trait;

use crate::agents::capabilities::AgentCapability;
use crate::security::command_policy::CommandPolicy;
use crate::types::agent_tool::{AgentTool, CallVerdict, RiskLevel};

/// Çocuk sürece geçirilmesine İZİN verilen ortam değişkenleri.
/// Geri kalan her şey (özellikle `*_API_KEY` gibi sırlar) temizlenir —
/// aksi halde agent `env` çalıştırıp sırları çıktıya, oradan da AI
/// provider'a geri gönderebilirdi.
const ENV_ALLOWLIST: &[&str] = &["PATH", "HOME", "LANG", "TMPDIR", "TERM"];

pub struct TerminalAgentTool {
    inner: Arc<AgentTerminalTool>,
    /// B4: argüman-bazlı karar (allow/ask/deny). Bkz. security::command_policy.
    policy: CommandPolicy,
    /// Komutların çalışma dizini + HOME'u. None → süreç cwd'si (eski
    /// davranış; Android'de "/" ve salt-okunur olduğundan işe yaramaz).
    workspace: Option<PathBuf>,
}

impl TerminalAgentTool {
    pub fn new() -> Self {
        Self::with_policy(CommandPolicy::load())
    }

    /// Belirli bir politikayla (testler / özel kurulum).
    pub fn with_policy(policy: CommandPolicy) -> Self {
        Self {
            inner: Arc::new(AgentTerminalTool::local()),
            policy,
            workspace: None,
        }
    }

    /// Varsayılan politika + belirli bir çalışma alanı (uygulamada kullanılan).
    /// Dizin yoksa oluşturulur.
    pub fn with_workspace(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        let mut tool = Self::new();
        tool.workspace = Some(dir);
        tool
    }

    /// B9: yol argümanları workspace dışına çıkıyorsa gerekçe.
    fn confinement_error(&self, arguments: &[String]) -> Option<String> {
        let ws = self.workspace.as_ref()?;
        crate::security::path_confinement::check_arguments(ws, arguments).err()
    }

    /// Ortamı temizleyip sadece allowlist'teki değişkenleri geçirir.
    fn sanitized_environment(&self) -> Environment {
        let mut env = Environment::new().clear();
        for key in ENV_ALLOWLIST {
            if let Ok(value) = std::env::var(key) {
                env = env.set(*key, value);
            }
        }
        // Çalışma alanı varsa HOME oraya işaret eder (`~`, git config vb.
        // uygulama sandbox'ında yazılabilir bir yere düşsün).
        if let Some(ws) = &self.workspace {
            env = env.set("HOME", ws.display().to_string());
        }
        env
    }
}

impl Default for TerminalAgentTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentTool for TerminalAgentTool {
    fn name(&self) -> &'static str {
        "terminal"
    }

    fn description(&self) -> &'static str {
        "Runs ONE program (no shell). arguments[0] is ONLY the program name (no spaces), \
         every other word is a SEPARATE array element: [\"touch\", \"deneme.txt\"], \
         [\"git\", \"status\"]. Never put a whole command line in one string. \
         Shell syntax like && ; | is NOT interpreted. Runs in the app workspace directory; file paths must stay inside it \
         (no absolute paths outside, no ..)."
    }

    fn required_capability(&self) -> Option<AgentCapability> {
        Some(AgentCapability::TerminalExecution)
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }

    /// B4: tool-bazlı sabit High yerine komutun KENDİSİNE göre karar:
    /// `sh -c` → Deny, `git status` → Allow, `git commit` → Ask.
    ///
    /// B9: workspace verilmişse yol argümanları kökün dışına çıkamaz;
    /// çıkıyorsa Deny (onayla bile çalışmaz).
    fn assess_call(&self, arguments: &[String]) -> Option<CallVerdict> {
        let verdict = self.policy.assess(arguments);
        if matches!(verdict, CallVerdict::Deny { .. }) {
            return Some(verdict);
        }
        if let Some(reason) = self.confinement_error(arguments) {
            return Some(CallVerdict::Deny { reason });
        }
        Some(verdict)
    }

    async fn invoke(&self, arguments: Vec<String>) -> Result<String, String> {
        // Onay beklerken dosya sistemi değişmiş olabilir (ör. yeni symlink):
        // çalıştırma anında yol denetimi yeniden yapılır.
        if let Some(reason) = self.confinement_error(&arguments) {
            return Err(format!("reddedildi: {reason}"));
        }
        let mut iter = arguments.into_iter();
        let program = iter
            .next()
            .ok_or_else(|| "terminal: program belirtilmedi (arguments[0] boş)".to_string())?;
        let args: Vec<String> = iter.collect();
        let program_name = program.clone();

        let command = CommandBuilder::new(program)
            .map_err(|e| e.to_string())?
            .args(args)
            .env(self.sanitized_environment())
            .build();

        let session = match self
            .inner
            .call(TerminalToolRequest::Create { cwd: self.workspace.clone() })
            .await
            .map_err(|e| e.to_string())?
        {
            TerminalToolResponse::SessionCreated(s) => s,
            _ => return Err("terminal: beklenmeyen yanıt (oturum oluşturma)".to_string()),
        };

        let result = self
            .inner
            .call(TerminalToolRequest::Execute {
                session_id: session.id,
                command,
            })
            .await;

        // Sonuç ne olursa olsun oturum temizlensin (sızıntı olmasın).
        let _ = self
            .inner
            .call(TerminalToolRequest::Remove {
                session_id: session.id,
            })
            .await;

        match result {
            Ok(TerminalToolResponse::Executed(process)) => {
                if process.status.success() {
                    Ok(process.output.stdout_string())
                } else {
                    let code = process
                        .status
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "sinyal".to_string());
                    Err(format!(
                        "komut {code} koduyla başarısız oldu: {}",
                        process.output.stderr_string()
                    ))
                }
            }
            Ok(_) => Err("terminal: beklenmeyen yanıt (çalıştırma)".to_string()),
            Err(e) => Err(explain_spawn_failure(&program_name, e.to_string())),
        }
    }
}

/// Süreç hiç başlatılamadıysa (program yok / yürütme izni yok) ham işletim
/// sistemi hatası ("Permission denied (os error 13)") kullanıcıyı yanıltır:
/// asıl sebep çoğu zaman yanlış/olmayan bir program adıdır (ör. doğal dil
/// cümlesi yazmak). Net bir açıklama ekler; diğer hatalara dokunmaz.
pub(crate) fn explain_spawn_failure(program: &str, raw: String) -> String {
    let lower = raw.to_lowercase();
    let spawn_failed = lower.contains("failed to spawn");
    let missing_or_denied = lower.contains("permission denied")
        || lower.contains("no such file")
        || lower.contains("not found");
    if spawn_failed && missing_or_denied {
        format!(
            "'{program}' programı başlatılamadı (bulunamadı ya da çalıştırma izni yok). \
             Bu bir komut satırıdır, doğal dil değil: gerçek bir program adıyla başla, \
             örn. ls, touch notlar.txt, echo merhaba. Ayrıntı: {raw}"
        )
    } else {
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_high_risk_and_terminal_capability() {
        let tool = TerminalAgentTool::new();
        assert_eq!(tool.name(), "terminal");
        assert_eq!(tool.risk_level(), RiskLevel::High);
        assert_eq!(
            tool.required_capability(),
            Some(AgentCapability::TerminalExecution)
        );
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn assess_call_uses_the_argument_policy() {
        let tool = TerminalAgentTool::with_policy(CommandPolicy::default_policy());
        let a = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(matches!(
            tool.assess_call(&a(&["sh", "-c", "id"])),
            Some(CallVerdict::Deny { .. })
        ));
        assert_eq!(tool.assess_call(&a(&["git", "status"])), Some(CallVerdict::Allow));
        assert!(matches!(
            tool.assess_call(&a(&["git", "commit"])),
            Some(CallVerdict::Ask { .. })
        ));
        assert!(matches!(tool.assess_call(&[]), Some(CallVerdict::Deny { .. })));
    }

    #[tokio::test]
    async fn empty_arguments_are_rejected() {
        let tool = TerminalAgentTool::new();
        let result = tool.invoke(vec![]).await;
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn runs_a_program_and_returns_stdout() {
        let tool = TerminalAgentTool::new();
        let out = tool
            .invoke(vec!["echo".to_string(), "merhaba".to_string()])
            .await
            .expect("echo çalışmalı");
        assert_eq!(out.trim(), "merhaba");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failing_command_reports_exit_code() {
        let tool = TerminalAgentTool::new();
        let err = tool
            .invoke(vec!["false".to_string()])
            .await
            .expect_err("`false` başarısız olmalı");
        assert!(err.contains("koduyla başarısız"), "hata: {err}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn shell_metacharacters_are_not_interpreted() {
        // `echo a && echo b` bir shell'e verilseydi iki satır basardı.
        // Shell yok: "&&" düz bir argüman olarak echo'ya gider.
        let tool = TerminalAgentTool::new();
        let out = tool
            .invoke(vec![
                "echo".to_string(),
                "a".to_string(),
                "&&".to_string(),
                "echo".to_string(),
                "b".to_string(),
            ])
            .await
            .unwrap();
        assert_eq!(out.trim(), "a && echo b");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn commands_run_inside_the_workspace_with_home_pointing_there() {
        let ws = std::env::temp_dir().join(format!("aetheros_ws_{}", uuid::Uuid::new_v4()));
        let tool = TerminalAgentTool::with_workspace(ws.clone());
        assert!(ws.is_dir(), "workspace oluşturulmalı");

        // cwd = workspace
        let out = tool.invoke(vec!["pwd".to_string()]).await.unwrap();
        assert_eq!(
            std::fs::canonicalize(out.trim()).unwrap(),
            std::fs::canonicalize(&ws).unwrap()
        );

        // yazılabilir: touch workspace'te dosya oluşturur
        tool.invoke(vec!["touch".to_string(), "deneme.txt".to_string()])
            .await
            .expect("touch workspace'te çalışmalı");
        assert!(ws.join("deneme.txt").is_file());

        // HOME workspace'e işaret eder
        let env_out = tool.invoke(vec!["env".to_string()]).await.unwrap();
        assert!(
            env_out.lines().any(|l| l == format!("HOME={}", ws.display())),
            "HOME workspace olmalı: {env_out}"
        );
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn secrets_in_environment_are_not_leaked_to_child() {
        // SAFETY: test sürecinde tek başına ayarlanan, benzersiz isimli değişken.
        unsafe {
            std::env::set_var("AETHEROS_TEST_SECRET_API_KEY", "super-secret-value");
        }

        let tool = TerminalAgentTool::new();
        // `env` çıktısında sır GÖRÜNMEMELİ (allowlist dışı → temizlendi).
        let out = tool.invoke(vec!["env".to_string()]).await.unwrap();

        assert!(
            !out.contains("super-secret-value"),
            "ortam sırrı çocuk sürece sızdı: {out}"
        );
        assert!(!out.contains("AETHEROS_TEST_SECRET_API_KEY"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn paths_outside_the_workspace_are_denied_and_never_run() {
        let ws = std::env::temp_dir().join(format!("aetheros_ws_{}", uuid::Uuid::new_v4()));
        let tool = TerminalAgentTool::with_workspace(ws.clone());
        let a = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        // politika onsuz Ask/Allow derdi; yol hapsi Deny'a çevirir
        for cmd in [
            a(&["cat", "/etc/passwd"]),
            a(&["ls", ".."]),
            a(&["touch", "../kacti.txt"]),
            a(&["ls", "/"]),
        ] {
            assert!(
                matches!(tool.assess_call(&cmd), Some(CallVerdict::Deny { .. })),
                "{cmd:?} Deny olmalı"
            );
            // onay atlansa bile invoke kendi başına reddeder
            let err = tool.invoke(cmd.clone()).await.expect_err("çalışmamalı");
            assert!(err.contains("reddedildi"), "{err}");
        }
        assert!(!ws.parent().unwrap().join("kacti.txt").exists());

        // içerideki yol hâlâ çalışır
        assert_eq!(tool.assess_call(&a(&["ls", "."])), Some(CallVerdict::Allow));
        tool.invoke(a(&["touch", "alt.txt"])).await.unwrap();
        assert!(ws.join("alt.txt").is_file());
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn without_a_workspace_no_confinement_is_applied() {
        // Çalışma alanı verilmemiş (eski/test davranışı): yol denetimi yok.
        let tool = TerminalAgentTool::with_policy(CommandPolicy::default_policy());
        let a = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(matches!(
            tool.assess_call(&a(&["cat", "/etc/hostname"])),
            Some(CallVerdict::Ask { .. })
        ));
    }
}
