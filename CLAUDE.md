# OP-3

- Todo lo que se ve en pantalla o se maneja con la MK3 sigue `docs/DESIGN.md` (Tratado de UX/UI). Leerlo antes de tocar `crates/op3-ui`, `crates/op3/src/controller.rs` o `docs/op3-twin.html`.
- Un cambio de UI se prueba primero en `docs/op3-twin.html`, y se actualiza `docs/DESIGN.md` en el mismo PR.
- Sin dependencias GPL (Encdr y autotel/maschine-mk3-linux son GPL-3.0): el protocolo MK3 está implementado aparte.
