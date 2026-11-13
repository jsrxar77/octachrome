# OP-3

- **Para seguir el trabajo, leer primero `docs/HANDOVER.md`** (estado, decisiones, próximos pasos).

- Todo lo que se ve en pantalla o se maneja con la MK3 sigue `docs/DESIGN.md` (Tratado de UX/UI). Leerlo antes de tocar `crates/op3-ui`, `crates/op3/src/controller.rs` o `docs/op3-twin.html`.
- Un cambio de UI se prueba primero en `docs/op3-twin.html`, y se actualiza `docs/DESIGN.md` en el mismo PR.
- Sin dependencias GPL (Encdr y autotel/maschine-mk3-linux son GPL-3.0): el protocolo MK3 está implementado aparte.
- Los instrumentos son el centro del producto: seguir `docs/INSTRUMENTOS.md` (máximo 8 parámetros, un dibujo que explique el sonido). Qué hace cada botón: `docs/BOTONES.md`.
- Código de Mutable Instruments (STM32, MIT) se puede usar, pero sin sus nombres de módulo. Surge XT y Vital son GPL: no incluir sin decisión de Javi.
