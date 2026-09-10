# Test fixture: run42.wasm

Elle bayt bayt inşa edilmiş, geçerli bir minimal WASM modülü:

```
(module
  (func (export "run") (result i32)
    i32.const 42))
```

Sıfır export'a sahip stres testi modülünün (`0061736d01000000`) aksine,
bu modül gerçek bir `run` export'una sahip ve wasmi'nin
`get_typed_func::<(), i32>` yolunu kullanarak başarıyla tamamlanması
beklenir (state=Completed).

Hex:
0061736d010000000105016000017f030201000707010372756e00000a06010400412a0b
