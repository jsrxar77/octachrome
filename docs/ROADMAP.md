# OP-3 — Roadmap

Este es el orden en que se construye OP-3. Sale de dos documentos: del [mapa de botones](BOTONES.md), que le asigna una idea a cada botón de la MK3, y de [INSTRUMENTOS.md](INSTRUMENTOS.md), según el cual los instrumentos son el centro del producto. Si llegás sin contexto, leé primero [HANDOVER.md](HANDOVER.md).

**Cómo leerlo.** Cada fase dice:

- **qué es**;
- **por qué importa**;
- **de qué depende**;
- su **estado**;
- **cuándo está terminada**.

**Estados:**

| Estado | Significa |
| :-- | :-- |
| ✓ hecho | Funciona y tiene tests o capturas |
| ◐ en curso / parcial | Una parte funciona |
| ○ pendiente | No se empezó |
| ⚠ sin probar | Está escrito y compila, pero no se probó en el equipo real |

**Reglas para todas las fases:**

- Todo lo que se ve o se toca se prueba primero en el Twin (`docs/op3-twin.html`) y después se porta a Rust. Sigue el [Tratado de UX/UI](DESIGN.md).
- Cuando cambia el estado de un botón, se actualizan [BOTONES.md](BOTONES.md) y este archivo en el mismo PR.
- Una fase está terminada cuando:
  - pasa `cargo test --workspace` y `cargo clippy`;
  - el CI está en verde;
  - el Twin muestra lo mismo que Rust;
  - si hay hardware de por medio, se probó en la MK3.

---

## 0. Base (hecho al 2026-11-13)

**Qué es.** Todo lo que existe hoy: el motor, el secuenciador, el driver de la MK3, el lenguaje visual, los primeros instrumentos y la documentación.

**Por qué importa.** Es el piso sobre el que se construye todo lo demás. Esta parte no debería cambiar mucho: el secuenciador y los botones quedan chicos y fijos.

| Ítem | Qué es | Estado |
| :-- | :-- | :-- |
| Motor de audio (`op3-engine`) | Hilo de audio a 48 kHz con 8 pistas, mezcla y Punch-in FX. Se comunica con la UI por colas sin bloqueos y no reserva memoria mientras suena | ✓ |
| Secuenciador (`op3-seq`) | Polimetría (cada pista de 1 a 16 pasos), *Step Components* (ratchet, chance, every, etc.) y el *Brain* (acordes que transponen las pistas enlazadas). Es determinista: el mismo proyecto da la misma salida, al sample | ✓ |
| Driver MK3 (`op3-hw`) | Pads, perillas, botones, LEDs y las dos pantallas por USB. Implementación propia, sin código GPL | ⚠ escrito y con tests, **nunca probado con una MK3 real** |
| Despliegue en la Pi (`deploy/`, `scripts/`) | Compilación cruzada, servicio systemd, reglas udev y prioridad de tiempo real | ⚠ sin probar en una Pi real |
| Lenguaje visual (`op3-gfx`, `op3-ui`) | 8 colores de perilla, tres temas (op-1, op-1 field, op-xy), letra Jost, dibujos con antialiasing y un overlay al tocar una perilla | ✓ |
| Instrumentos | KickSyn (kick), FmBass (bajo FM), MidiOutBridge (MIDI hacia afuera) y **macro** (24 motores tomados de Plaits, MIT) | ✓ |
| Botones funcionando | STEP, CHORDS (Brain), PERFORM (Punch-in), grupos A–H, MUTE, ERASE + pad, PLAY, STOP, RESTART, SHIFT y botones de pantalla 5–6 | ✓ |
| Twin | Gemelo en HTML con las mismas pantallas y sonido aproximado en WebAudio | ✓ |
| Documentos | [DESIGN.md](DESIGN.md), [BOTONES.md](BOTONES.md), [INSTRUMENTOS.md](INSTRUMENTOS.md) y [HANDOVER.md](HANDOVER.md) | ✓ |
| CI | Tests, clippy, simulador, chequeo para la Pi (aarch64) y build en macOS | ✓ |

---

## 1. Prueba en hardware — **lo próximo**

**Qué es.** Conectar la MK3 al Mac de Javi y correr `cargo run --release -- hw-test`. Ese comando pinta un arcoíris en los pads, muestra un frame de prueba en las dos pantallas e imprime cada evento (pad, perilla, botón).

**Por qué importa.** Todo el driver se escribió a partir de documentación de terceros, sin el equipo enfrente. Si el formato de las pantallas o el de los colores está mal, nada de lo visual se va a ver en la MK3. Es el riesgo más grande del proyecto, y conviene sacárselo de encima antes de construir más UI.

**Depende de.** Que Javi tenga la MK3 conectada. Antes hay que cerrar los servicios de Native Instruments (`NIHardwareAgent`), que si no se quedan con la pantalla.

**Estado.** ○ pendiente.

| Ítem | Qué se mira | Si falla, dónde se corrige |
| :-- | :-- | :-- |
| Pantallas | El frame de prueba aparece entero y sin cortes | `op3-hw/src/mk3/protocol.rs`, `encode_screen`. Hay un formato alternativo documentado en el README |
| Colores | La primera barra del frame de prueba se ve **roja** | El orden de bytes RGB565 en el mismo archivo |
| Pads | Los 16 pads se iluminan, y al tocarlos se imprime el número correcto con su velocidad | La tabla de pads en `protocol.rs` |
| LEDs | Los colores de los LEDs se parecen a los pedidos (solo el rojo y el naranja están confirmados) | `NiColor` en `op3-hw` |
| Perillas y botones | Cada uno imprime el nombre correcto | Los mapas de `protocol.rs` |

**Terminado cuando.** Las cinco filas dan bien y `cargo run --release` se puede tocar con la MK3: suena, se ve y los LEDs responden.

---

## 2. Esqueleto de navegación

**Qué es.** Que todos los botones de pantalla y de navegación hagan lo que dice [BOTONES.md](BOTONES.md), y que cada pantalla muestre arriba qué hace cada botón (regla 11 del tratado).

**Por qué importa.** Hoy varias funciones se alcanzan por atajos (por ejemplo, mantener un pad abre el inspector). Sin un esqueleto claro, cada función nueva inventa su propia forma de llegar a ella y la máquina se vuelve confusa. Esta fase fija el "dónde está cada cosa" antes de sumar funciones.

**Depende de.** Fase 1, para confirmar que los botones llegan bien desde el hardware. Se puede adelantar en el Twin y en el simulador.

**Estado.** ◐ parcial: los botones de pantalla 5–6 ya funcionan; el resto no.

| Ítem | Qué es | Estado |
| :-- | :-- | :-- |
| Franja superior | Sobre cada pantalla, el rótulo del botón que está encima, en la misma columna de 120 px. Un botón sin función no muestra nada | ○ |
| Botones de pantalla 1–4 | **step** (grilla de pasos), **components** (inspector del paso), **players** (arpegio y acordes), **bar** (largo, división, cuantización) | ◐ las vistas existen, pero los botones todavía no las abren |
| Botones de pantalla 5–8 | **sound** (página 1 del instrumento), **shape** (página 2), **fx** y **mod** (los de la fase 5) | ◐ 5–6 hechos |
| PAGE ◀ ▶ | Página anterior y siguiente del modo actual | ○ |
| Encoder 4D | Moverse en listas e inspector (▲▼◀▶), confirmar (push) y cambiar el valor elegido (girar) | ◐ girar cambia el tempo; lo demás falta |
| SETTINGS | Elegir el tema, la salida de audio y MIDI y el brillo, desde la MK3 | ○ (hoy el tema se elige por línea de comandos: `-t op-xy`) |

**Terminado cuando.** Cada botón de esta tabla hace lo que dice BOTONES.md, la franja superior se ve en las dos pantallas y en el Twin, y se puede llegar a cualquier vista sin atajos ocultos.

---

## 3. Tocar y grabar

**Qué es.** Poder tocar notas en vivo con los pads y grabarlas en el secuenciador, más los controles básicos de tiempo.

**Por qué importa.** Hoy las notas se escriben paso a paso. Un groovebox tiene que poder tocarse y grabar lo tocado, como en el OP-XY.

**Depende de.** Fase 2: KEYBOARD necesita el encoder 4D para la octava, y REC necesita la vista de pasos.

**Estado.** ○ pendiente.

| Ítem | Qué es | Estado |
| :-- | :-- | :-- |
| KEYBOARD | Los 16 pads funcionan como teclado cromático de la pista elegida; ◀ ▶ del encoder 4D cambia la octava | ○ |
| REC | Graba en los pasos lo que se toca en los pads; con Shift, cuenta 1 compás antes | ○ |
| TAP + metrónomo | Marcar el tempo con golpes; Shift + TAP prende el metrónomo | ○ |
| FIXED VEL | Todas las notas con la misma velocidad | ○ |
| TEMPO, SWING y VOLUME | Mantener el botón y girar el encoder 4D | ○ |
| AUTO | Mantener AUTO y girar una perilla graba ese movimiento en la pista (automatización) | ○ |

**Terminado cuando.** Se puede grabar una línea en KEYBOARD con REC, escucharla en loop y verla en la grilla de pasos, y la prueba del simulador lo confirma.

---

## 4. Instrumentos — **lo central**

**Qué es.** Hacer crecer la colección de instrumentos y el mecanismo para agregarlos. Es el corazón del producto, según [INSTRUMENTOS.md](INSTRUMENTOS.md).

**Por qué importa.** Javi quiere un OP-XY al que se le puedan sumar instrumentos. Cada uno tiene como máximo 8 perillas, un dibujo que explica el sonido y tiene que sonar bien con cualquier combinación. Cuantos más buenos instrumentos haya, más vale OP-3.

**Depende de.** PLUG-IN necesita la fase 2 (navegación por listas). Los instrumentos nuevos no dependen de nada y se pueden hacer en paralelo.

**Estado.** ◐ en curso: macro ya está hecho.

| Ítem | Qué es | Estado |
| :-- | :-- | :-- |
| PLUG-IN | Elegir el instrumento de cada pista desde una lista en la pantalla derecha | ○ (la lista ya existe en código: `builtin_names()` y `create()` en `op3-cartridges`) |
| **resonator** | Cuerdas, campanas y tubos con un modelo físico, tomado de Rings (MIT). Se vendoriza igual que macro, en un crate `op3-rings` | ○ |
| **drums** | Un instrumento con 8 sonidos de batería, disparados con los pads 9–16 en PAD MODE | ○ |
| **sampler** | Tocar un sample con el teclado (se graba en la fase 7) | ○ |
| Kit de dibujos | Piezas reusables en `op3-gfx` (osciloscopio, envolvente, onda polar, teclado) para que un instrumento nuevo se vea bien sin diseñar todo desde cero | ○ (hoy cada instrumento dibuja todo por su cuenta) |
| Instrumentos como paquete aparte | Compilar un instrumento como `.so` o `.dylib`, copiarlo a `cartridges/` y que OP-3 lo cargue al arrancar, sin recompilar OP-3 (etapa 2 de INSTRUMENTOS.md) | ○ |
| Presupuesto de CPU | Medir en la Pi 5 cuánto usa cada instrumento. El objetivo es que 8 pistas sonando usen menos del 60 % de un núcleo | ○ (depende de tener una Pi) |

**Terminado cuando.** Se puede elegir cualquiera de los instrumentos desde la MK3, y cada uno pasa los tests de contrato de `op3-cartridges/src/lib.rs`, tiene su dibujo en Rust y en el Twin, y cumple las 5 reglas de INSTRUMENTOS.md.

---

## 5. Sonido de la pista

**Qué es.** Todo lo que le da forma al sonido de una pista además del instrumento: presets, un efecto, una modulación y el arpegiador.

**Por qué importa.** Así cada instrumento se mantiene simple (8 perillas) y la pista suma lo demás de la misma forma para todos, como en el OP-XY.

**Depende de.** Fase 2 (botones 3, 7 y 8) y del guardado de estado de los instrumentos (`save_state`, que ya existe).

**Estado.** ○ pendiente.

| Ítem | Qué es | Estado |
| :-- | :-- | :-- |
| BROWSER | Guardar y cargar presets de cada instrumento | ○ |
| fx (botón 7) | Un efecto por pista con 4 perillas (por ejemplo delay, reverb, distorsión) | ○ |
| mod (botón 8) | Un LFO por pista que mueve una perilla del instrumento | ○ |
| MACRO (Auxiliar) | Envíos a efectos compartidos y ajustes del Brain | ○ |
| NOTE REPEAT y arpegiador | Repetir la nota que se mantiene; con Shift, arpegiar el acorde (botón 3, players) | ○ (hoy NOTE REPEAT abre Punch) |

**Terminado cuando.** Cualquier pista tiene fx y mod con 4 perillas cada uno, se pueden guardar y cargar presets, y el arpegiador sigue al Brain.

---

## 6. Estructura

**Qué es.** Pasar de un loop a una canción: varios patrones por pista, escenas, arreglo, y guardar y abrir proyectos.

**Por qué importa.** Sin esto, OP-3 solo toca un loop y se pierde todo al apagarlo.

**Depende de.** Fase 3 (grabar) y de un formato de proyecto en disco. Guardar proyectos (FILE) se podría adelantar porque es independiente.

**Estado.** ○ pendiente.

| Ítem | Qué es | Estado |
| :-- | :-- | :-- |
| FILE | Guardar, abrir y crear proyectos. Shift + FILE guarda | ○ |
| PATTERN | 16 patrones por pista, elegidos con los pads | ○ |
| SCENE y ARRANGER | Escenas (un patrón por pista) y una cadena de escenas que forma la canción | ○ |
| DUPLICATE, SELECT, SOLO | Copiar pasos, patrones o pistas; elegir sin disparar; aislar una pista | ○ |
| VARIATION, EVENTS, LOCK | Variación aleatoria, edición de eventos e instantáneas de perillas | ○ |
| STOP ×2 | Apretar STOP dos veces apaga todas las notas | ◐ (falta la segunda pulsación) |
| ERASE + grupo | Vaciar una pista entera | ◐ (ERASE + pad ya funciona) |

**Terminado cuando.** Se puede armar una canción con varias escenas, guardarla, apagar y volver a abrirla igual.

---

## 7. Mezcla y samples

**Qué es.** La mezcla final y la grabación de samples.

**Por qué importa.** Cierra el ciclo de hacer un tema completo en la máquina. El sampler necesita que exista la grabación.

**Depende de.** Fase 4 (el instrumento sampler) y de la entrada de audio de la placa.

**Estado.** ○ pendiente.

| Ítem | Qué es | Estado |
| :-- | :-- | :-- |
| MIXER | Nivel y paneo de las 8 pistas, EQ y compresor en el master | ○ |
| SAMPLING | Grabar un sample, recortarlo y mandarlo al sampler | ○ |
| Smart Strip | Elegir con PITCH, MOD o NOTES qué hace la tira táctil | ◐ (hoy siempre es un filtro master) |

**Terminado cuando.** Se puede grabar un sample, tocarlo en una pista y mezclar las 8 pistas desde la MK3.

---

## Pendientes técnicos (sin fase fija)

Mejoras internas que no cambian lo que ve el usuario. Se hacen cuando molestan o cuando una fase las necesita.

- **Parameter locks con precisión de sample.** Hoy se aplican al principio del bloque de audio, hasta 2,7 ms antes de tiempo.
- **Snapshot de la UI de cada instrumento.** Que dibujar nunca comparta el mutex con el audio, para evitar cortes.
- **Prioridad de tiempo real** para el hilo de audio en Linux, y medir la latencia en la Pi 5.
- **MIDI con marca de tiempo.** Hoy sale apenas se procesa, con hasta 1 ms más un bloque de demora.
- **Escribir la progresión del Brain desde los pads.** Hoy los pads solo tocan acordes en vivo.
