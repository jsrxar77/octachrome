# OP-3 — Handover

Documento para quien siga el trabajo (persona o IA). Leelo primero, después `CLAUDE.md` y los cuatro documentos de diseño. Estado al **2026-11-13**.

## 1. Qué es OP-3

Un groovebox standalone en Rust. Corre en una **Raspberry Pi 4/5** y se desarrolla en **macOS Apple Silicon**. El único control es una **Native Instruments Maschine MK3**: pads, 8 perillas, Smart Strip, LEDs y dos pantallas de 480×272. El dueño del producto es **Javi**, que escribe en español rioplatense; respondele en ese idioma.

El modelo es el OP-XY de Teenage Engineering: un secuenciador con polimetría, *Step Components*, una pista de armonía (*Brain*) y *Punch-in FX*. Lo central es otra cosa: **los instrumentos son el producto**. El secuenciador y los botones quedan chicos y fijos; lo que crece es la colección de instrumentos ("cartridges"), cada uno con pocas perillas y un dibujo que explica el sonido.

## 2. Documentos que mandan (en este orden)

| Documento | Qué decide |
| :-- | :-- |
| `CLAUDE.md` | Reglas del repo para cualquier IA |
| `docs/DESIGN.md` | Tratado de UX/UI: las 13 reglas, colores, letra y layout. **Si el código no coincide, el que está mal es el código** |
| `docs/INSTRUMENTOS.md` | Qué es un instrumento, sus reglas (máx. 8 parámetros, un dibujo), el mecanismo de extensión y los candidatos |
| `docs/BOTONES.md` | Qué hace cada botón de la MK3 (Maschine → OP-XY → OP-3) y su estado ✓ ◐ ○ |
| `docs/ROADMAP.md` | Orden de trabajo |
| `docs/op3-twin.html` | **OP-3 Twin**: gemelo interactivo en HTML/WebAudio. Todo cambio de UI se prueba acá primero |

## 3. Decisiones tomadas (no reabrir sin Javi)

- **Licencia MIT o Apache-2.0.** No se agregan dependencias GPL. Encdr y autotel/maschine-mk3-linux son GPL-3.0: del protocolo MK3 se tomaron los hechos, no el código. Surge XT y Vital son GPL y quedan fuera salvo que Javi decida otra cosa.
- **Motores de síntesis de Mutable Instruments (MIT).** Javi eligió empezar por ahí. El código se puede usar, pero **sin los nombres de los módulos**, porque son marca registrada: el de Plaits se llama "macro", el de Rings se va a llamar "resonator", etc. Hay que dar crédito en el README del vendor.
- **Estética:** el lenguaje del OP-1 y del OP-XY (color ligado a cada control físico, dibujos en vez de texto, fondo negro absoluto). Tomamos las ideas, nunca sus gráficos ni sus fuentes. No tiene que "parecer un proyecto de programación".
- **8 perillas, 8 colores.** Las perillas 1–4 manejan el secuenciador y se ven en la pantalla izquierda. Las 5–8 manejan el instrumento y se ven en la derecha. Hay tres temas: op-1 (por defecto), op-1 field y op-xy.
- **Letra: Jost** (OFL), en pesos 300 y 400, ajustada a ojo para parecerse a la del OP-XY. FlipMono no se puede usar: en realidad es SF Mono, con licencia solo para equipos Apple.
- **Instrumentos:** como máximo 8 parámetros, 4 de sonido (botón 5) y 4 de forma (botón 6). Efectos y modulación son por pista e iguales para todos los instrumentos.

## 4. Arquitectura

Es un workspace de Cargo; el detalle está en el `README.md`.

| Crate | Qué hace |
| :-- | :-- |
| `op3-gfx` | Framebuffer RGB565 480×272 y dirty rects. Primitivas con antialiasing (`paint.rs`), letra Jost con fontdue (`font.rs`) y temas y colores de perilla (`theme.rs`: `theme()`, `knob(i)`) |
| `op3-cartridge` | El contrato `InstrumentCartridge` (`process`, `parameters`, `secondary_parameters`, `draw_ui`, `save_state`/`load_state`…) |
| `op3-seq` | Secuenciador determinista: 96 PPQN, polimetría, Step Components, Brain |
| `op3-engine` | Hilo de audio: secuenciador, 8 slots, mezcla, Punch-in FX y filtro de la Smart Strip. Comunicación lock-free con `rtrb` |
| `op3-plaits` | El oscilador macro de Plaits en C++, vendorizado **sin cambios** (`vendor/`, eurorack f4cfa76 y stmlib d18def8). Se compila con `cc` (`build.rs`, `-DTEST -include cstdio`) detrás de un shim en C (`src/shim.cc`). Trae 24 motores |
| `op3-cartridges` | KickSyn, FmBass, MidiOutBridge y **macro** (`macro_osc.rs`). La lista de instrumentos está en `builtin_names()` / `create()` de `lib.rs`. Los tests de contrato que todo instrumento tiene que pasar también están en `lib.rs` |
| `op3-hw` | `ControlSurface`, `MockSurface` y el protocolo MK3 como funciones puras. El transporte USB va con la feature `mk3` (hidapi o hidraw, más nusb) |
| `op3-ui` | Pantalla izquierda (`left.rs`), derecha (`right.rs`), pie de diales y overlay de toque (`theme.rs`), animación (`anim.rs`) |
| `op3` | Binario: controlador, bucle de 60 fps, audio con cpal, MIDI, `sim`, `hw-test` y el proyecto demo (`demo.rs`) |

## 5. Compilar, correr y probar

Hace falta Rust estable y un compilador de C++. En Mac: `xcode-select --install`. En Linux o en la Pi: `build-essential` y `libasound2-dev`.

```sh
cargo test --workspace                 # 81 tests, sin hardware
cargo clippy --workspace --all-targets
cargo run --release -- sim -s 8 -o out # MK3 simulada: out.wav + capturas .ppm (1-step … 6-macro)
cargo run --release -- hw-test         # con la MK3 real
cargo run --release [-- -t op-xy]      # tocar
open docs/op3-twin.html                # el Twin
```

El CI (`.github/workflows/ci.yml`) corre en Linux (tests, clippy, simulador y `cargo check` para aarch64, con `g++-aarch64-linux-gnu` porque op3-plaits compila C++) y en macOS. Para la Pi están `scripts/build-pi.sh` y `deploy/` (systemd, udev, setup).

## 6. Estado

**Hecho**
- Motor, secuenciador con polimetría, Step Components, Brain y Punch-in FX.
- Driver MK3 completo (pads, perillas, botones, LEDs, pantallas), `hw-test` y despliegue en la Pi.
- Lenguaje visual: 8 colores, 3 temas, Jost, primitivas con antialiasing, dibujo animado por instrumento, overlay al tocar una perilla.
- Instrumentos: KickSyn, FmBass, MidiOutBridge y **macro**. Macro tiene 24 motores de Plaits, `engine`/`harmonics`/`timbre`/`morph` en la página 1 y `decay`/`color`/`variant`/`level` en la 2. Es monofónico y usa semántica de gate (los motores FM de 6 operadores necesitan el gate sostenido). Se duerme y da ceros exactos cuando no suena.
- Botones ✓: STEP, CHORDS, PERFORM, grupos A–H, MUTE, ERASE + pad, PLAY, RESTART, SHIFT y pantalla 5–6. El resto está en `docs/BOTONES.md`.
- Twin con las mismas pantallas, macro en la pista D y un sonido aproximado en WebAudio.

**Sin probar en hardware.** Nada se probó todavía con una MK3 real. Lo primero es que Javi corra `op3 hw-test` en su Mac. Hay que revisar:
1. El formato de transferencia de pantalla (`op3-hw/src/mk3/protocol.rs`, `encode_screen`).
2. El orden de colores RGB565: la primera barra del frame de prueba tiene que verse roja.
3. La paleta de LEDs más allá del rojo y el naranja.
4. Que en macOS los servicios de NI (`NIHardwareAgent`) no tengan tomada la interfaz USB 5.

Tampoco se midió el consumo de CPU en la Pi 5. El objetivo está en `INSTRUMENTOS.md` §5.

**PR abierto:** todo esto está en la rama `scaffold/engine`, en el PR #1 contra `main`. El CI está en verde.

## 7. Próximos pasos (en este orden)

1. **Prueba en hardware** con `hw-test`, y corregir lo que haga falta en el protocolo.
2. **Navegación:** la franja superior con los rótulos de los botones de pantalla 1–8 (regla 11), PAGE ◀▶, el encoder 4D y SETTINGS para elegir el tema desde la MK3.
3. **PLUG-IN:** elegir el instrumento de cada pista con la lista de `builtin_names()`.
4. **Más instrumentos:** resonator (desde Rings), drums (8 sonidos en los pads), sampler. Cada uno cumple `INSTRUMENTOS.md`: máximo 8 perillas, un dibujo sin texto y que cualquier combinación suene bien.
5. Un kit de dibujos reusables en `op3-gfx` (osciloscopio, envolvente, onda polar).
6. Instrumentos como paquete aparte (`.so`/`.dylib` en `cartridges/`), la etapa 2 del mecanismo de extensión.
7. Lo demás está en `docs/ROADMAP.md`: tocar y grabar, sonido de la pista (fx/mod), estructura, mezcla.

## 8. Cómo trabajar acá

- Un cambio de UI se prueba primero en `docs/op3-twin.html` y se actualiza `docs/DESIGN.md` en el mismo PR. Un cambio al tratado lo aprueba Javi.
- Cuando cambia algo, se actualizan `ROADMAP.md`, `BOTONES.md` o `INSTRUMENTOS.md` según corresponda.
- En pantalla todo va en minúsculas y sin códigos ni IDs (regla 8). Cada perilla pinta solo lo que mueve, en su color (regla 1).
- En el hilo de audio no se reserva memoria. Los instrumentos tienen que pasar los tests de contrato de `op3-cartridges/src/lib.rs`.
- Los tests que tocan el tema global usan `THEME_LOCK` (`op3-ui`), para que no fallen cuando corren en paralelo.
- Para vendorizar código de terceros: copiarlo sin cambios, con su licencia y un README de origen, como en `crates/op3-plaits/vendor/`.
