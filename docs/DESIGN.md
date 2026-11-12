# OP-3 — Tratado de UX/UI

Este documento manda sobre todo lo que OP-3 muestra y sobre cómo se maneja. Si el código o el Twin no coinciden con este tratado, está mal el código. Si hace falta algo que el tratado no cubre, primero se escribe acá, se aprueba y después se programa.

La dirección es el lenguaje de Teenage Engineering (OP-1, OP-1 field, OP-XY): color ligado al control físico, dibujos en vez de texto, negro absoluto. Tomamos las ideas, no sus gráficos, íconos ni tipografía. Parte del sistema "Impeccable" de Flip Tracker+ (`jsrxar77/flip-tracker`, `DESIGN.md`), adaptado a la Maschine MK3.

La referencia viva es **OP-3 Twin** (`docs/op3-twin.html`). Todo diseño nuevo se prueba ahí antes de portarse a `crates/op3-ui`.

---

## Las 10 reglas

1. **Ocho encoders, ocho colores.** Cada encoder tiene un color propio y fijo. Ese color pinta en pantalla, y solo eso, lo que el encoder mueve.
2. **1–4 tiempo, 5–8 sonido.** Los encoders 1 a 4 manejan el secuenciador y se ven en la pantalla izquierda. Los encoders 5 a 8 manejan el instrumento de la pista elegida y se ven en la pantalla derecha. Nunca se cruzan.
3. **Gris es contexto.** Lo que no se puede mover ahora va en gris.
4. **Negro absoluto.** El fondo es siempre `#000`.
5. **Dibujo antes que texto.** Cada vista tiene un dibujo que se mueve con el sonido o el valor. El texto es solo el nombre corto del parámetro y su valor.
6. **Calmo hasta que se activa.** Lo inactivo es línea fina; lo activo (paso en edición, pista elegida, encoder tocado) se ilumina.
7. **Tocar es preguntar.** Al tocar un encoder (sensor capacitivo) aparece en grande qué es y cuánto vale, en su color. Al soltarlo, desaparece. No hay instrucciones en pantalla.
8. **Minúsculas y sin código.** `decay`, `fmbass`, `126ms`. Nada de corchetes, guiones bajos, IDs ni `0x`.
9. **Una letra.** Jost, con antialiasing, en la escala fija de la sección 3. Nunca fuentes pixeladas de terminal.
10. **La pista es un punto.** El color de pista (A–H) identifica la pista en los pads y LEDs físicos, y en pantalla solo como punto, aro o relleno de pad. Nunca compite con los colores de encoder.

11. **Arriba botones, abajo perillas.** La franja superior de cada pantalla muestra qué hace cada botón que tiene encima; el pie, qué hace cada perilla que tiene debajo. Siempre en la misma columna de 120 px que el control físico. Los rótulos de botones van en blanco o gris, nunca en color de perilla. Un botón sin función no muestra nada.
12. **Cada botón significa una sola idea.** Lo que hace un botón de la MK3 en OP-3 se parece a lo que hace su equivalente en el OP-XY y a lo que dice su serigrafía. Shift da la función secundaria, nunca otra idea distinta.

---

## 1. Mapa de controles

Cada botón de la MK3 con su función en el software Maschine (según el [manual de NI](https://docs.native-instruments.com/online-guides/maschine-mk3-manual/en/quick-reference)), su idea equivalente en el OP-XY (según la [guía de TE](https://teenage.engineering/guides/op-xy/layout)) y lo que hace en OP-3. **Estado:** ✓ hecho, ◐ parcial, ○ por hacer (ver `docs/ROADMAP.md`).

### 1.1 Modos principales (los equivalentes de instrument, auxiliary, arrange y mix del OP-XY)

La pantalla izquierda siempre muestra tiempo y la derecha siempre muestra sonido. Un modo principal cambia qué módulos ofrecen los botones de pantalla 1–8, igual que M1–M4 en el OP-XY.

| Botón MK3 | En Maschine | En OP-XY | En OP-3 | Estado |
| :-- | :-- | :-- | :-- | :-- |
| CHANNEL | Propiedades del canal | instrument | **Instrumento**: el modo por defecto. Izquierda secuenciador, derecha cartridge de la pista | ◐ |
| PLUG-IN | Plug-ins del canal | instrument (motor) | Elegir el cartridge de la pista (lista de cartridges en la derecha) | ○ |
| MACRO | Macros | auxiliary | **Auxiliar**: Brain, envíos de efectos y entradas/salidas | ○ |
| ARRANGER | Ideas y canción | arrange | **Arreglo**: escenas y cadena de canción | ○ |
| MIXER | Mezcla | mix | **Mezcla**: nivel y paneo de las 8 pistas, EQ y compresor master | ○ |
| BROWSER | Navegador | (presets) | Presets del cartridge de la pista | ○ |
| SAMPLING | Editor de samples | sample | Grabar y recortar un sample para un cartridge sampler | ○ |
| FILE · Shift: guardar | Archivos | projects | Proyectos: abrir, nuevo, copiar. Shift + FILE guarda | ○ |
| SETTINGS | Preferencias | com | Ajustes: tema, audio, MIDI, brillo | ○ |
| AUTO · Shift: fijar | Grabar modulación | record + encoder | Mantener AUTO y girar una perilla graba automatización en la pista; con Shift queda fijo | ○ |
| PAGE ◀ ▶ | Página de parámetros | M1–M4 | Página anterior y siguiente del modo actual (lo mismo que los botones de pantalla) | ○ |

### 1.2 Botones de pantalla 1–8 (encima de las pantallas)

Son los módulos del modo actual, como M1–M4 del OP-XY. Su nombre se ve en la franja superior (regla 11). En modo Instrumento:

| Botón | Módulo | En OP-XY | Estado |
| :-- | :-- | :-- | :-- |
| 1 | step: grilla de pasos y polimetría | seq | ◐ (vista hecha, botón sin asignar) |
| 2 | components: inspector del paso | step components | ◐ (hoy se abre manteniendo un pad) |
| 3 | players: arpegio y acordes sobre la pista | players | ○ |
| 4 | bar: largo, división y cuantización | bar | ◐ (hoy en las perillas 1–2) |
| 5 | sound: página 1 del cartridge | M1 | ✓ |
| 6 | shape: página 2 del cartridge (envolventes) | M2 | ✓ |
| 7 | fx: efecto de la pista | M3 | ○ |
| 8 | mod: LFO de la pista | M4 | ○ |

### 1.3 Pads y sus modos

| Botón MK3 | En Maschine | En OP-XY | En OP-3 | Estado |
| :-- | :-- | :-- | :-- | :-- |
| STEP | Secuenciador por pasos | seq (pasos) | Pads = 16 pasos de la pista. Mantener un pad abre sus components | ✓ |
| KEYBOARD | Pads cromáticos | teclado | Pads = teclado cromático de la pista. Encoder 4D ◀ ▶ = octava (el − y + del OP-XY) | ○ |
| CHORDS | Modo acordes | players (chords) / brain | **Brain**: progresión de acordes. Pads = grados; mantener uno = acorde en vivo | ✓ |
| PAD MODE | Pads como sonidos | (tracks) | Pads 1–8 disparan las 8 pistas; 9–16 quedan para un cartridge de batería | ○ |
| PERFORM · Shift: elegir | Perform FX en la Smart Strip | punch-in fx | **Punch-in FX**: cada pad un efecto, la presión es la intensidad | ✓ |
| NOTE REPEAT · Shift: arp | Repetición y arpegiador | players (arp) | Repetir la nota mantenida; con Shift, arpegiador | ○ (hoy abre Punch) |
| FIXED VEL | Velocidad fija | — | Velocidad fija al grabar o tocar | ○ |

### 1.4 Edición

| Botón MK3 | En Maschine | En OP-XY | En OP-3 | Estado |
| :-- | :-- | :-- | :-- | :-- |
| Grupos A–H | Grupos | pistas 1–8 | Elegir pista. Shift + grupo la conecta o desconecta del Brain | ✓ |
| SELECT | Elegir sin disparar | — | Mantener SELECT + pad o grupo elige sin sonar | ○ |
| MUTE | Silenciar | shift + pista | Mantener MUTE + grupo silencia la pista | ✓ |
| SOLO | Solo | — | Mantener SOLO + grupo aísla la pista | ○ |
| PATTERN | Patrones | patrones por pista | Pads = 16 patrones de la pista | ○ |
| SCENE | Escenas | escenas (arrange) | Pads = 16 escenas | ○ |
| EVENTS | Editar eventos | step components | Abre components del paso seleccionado (igual que botón 2) | ○ |
| VARIATION | Humanizar y aleatorio | component random | Agrega variación aleatoria a la pista (velocidad, chance) | ○ |
| DUPLICATE | Duplicar | copiar | Mantener + origen + destino: copia paso, patrón o pista | ○ |
| ERASE · Shift: replace | Borrar | — | Mantener ERASE + pad borra el paso; + grupo vacía la pista | ◐ (pad sí, grupo no) |
| LOCK · Shift: ext | Instantánea de parámetros | — | Guardar y recuperar el estado de todas las perillas | ○ |

### 1.5 Transporte y tempo

| Botón MK3 | En Maschine | En OP-XY | En OP-3 | Estado |
| :-- | :-- | :-- | :-- | :-- |
| PLAY | Reproducir | play | Play / pausa | ✓ |
| STOP | Parar | stop (2 veces: silencio) | Parar; dos veces apaga todas las notas | ◐ (falta la segunda pulsación) |
| REC · Shift: count-in | Grabar | record | Grabar notas tocadas en los pads y automatización | ○ |
| RESTART · Shift: loop | Desde el principio | play (2.ª vez) | Volver al paso 1 y seguir | ✓ |
| TAP · Shift: metro | Tap tempo | tempo / metrónomo | Tap tempo; Shift activa el metrónomo | ○ |
| FOLLOW · Shift: grid | Seguir la reproducción | bar | Cuantización de la grabación | ○ |
| TEMPO + encoder 4D | Tempo | tempo | Tempo; Shift = fino | ○ |
| SWING + encoder 4D | Swing | tempo (swing) | Swing global | ○ |
| VOLUME + encoder 4D | Volumen | volumen | Volumen master; mantener un grupo = volumen de esa pista | ○ |
| SHIFT | Funciones secundarias | shift | Funciones secundarias (regla 12) | ✓ |

### 1.6 Otros controles

| Control | En OP-3 | Estado |
| :-- | :-- | :-- |
| Perillas 1–4 | Lo que muestra el pie izquierdo (secuenciador o el módulo abierto) | ✓ |
| Perillas 5–8 | Lo que muestra el pie derecho (cartridge) | ✓ |
| Tocar una perilla | Overlay grande con su parámetro | ✓ |
| Encoder 4D: girar | Valor del elemento elegido; si no hay, tempo | ◐ |
| Encoder 4D: ▲ ▼ ◀ ▶ y push | Moverse en listas y en el inspector; push confirma | ○ |
| Smart Strip | Según PITCH, MOD o PERFORM: pitch bend, modulación o intensidad del punch-in. Hoy: filtro master | ◐ |
| PITCH / MOD / NOTES | Modo de la Smart Strip | ○ |

## 2. Color

Tres temas. La lógica es la misma en los tres; cambia solo la paleta. Los valores hex son nuestros, no medidos de los equipos.

### 2.1 Colores de encoder

| Encoder | op-1 (por defecto) | op-1 field | op-xy |
| :-- | :-- | :-- | :-- |
| 1 | `#0099FF` azul | `#3B8EBA` azul acero | `#94A8C6` plata hielo |
| 2 | `#00E56A` verde | `#D9822B` ocre | `#00F076` verde señal |
| 3 | `#F2F4F8` blanco | `#A9AFB8` gris | `#EEF0F5` platino |
| 4 | `#FF4F00` naranja | `#F2652A` naranja | `#FF3844` rojo |
| 5 | `#8B6CFF` violeta | `#7FB58A` salvia | `#FFD60A` amarillo |
| 6 | `#FFD21F` amarillo | `#E3C77A` arena | `#34D3E8` cian |
| 7 | `#2ED3E6` cian | `#A88BC7` lila | `#9D7BFF` violeta |
| 8 | `#FF4FA0` rosa | `#D9677A` rosa viejo | `#FF8A1F` naranja |

Los cuatro primeros siguen el orden de los encoders de cada equipo de referencia (OP-1 original: azul, verde, blanco, naranja; OP-1 field: azul, ocre, gris, naranja). Los cuatro de instrumento se eligen para que no se confundan con los primeros a simple vista.

### 2.2 Colores base

| Token | op-1 | op-1 field | op-xy | Uso |
| :-- | :-- | :-- | :-- | :-- |
| `bg` | `#000000` | `#000000` | `#000000` | Fondo |
| `fg` | `#F2F4F8` | `#F4EFE6` | `#EEF0F5` | Texto y trazos principales |
| `dim` | `#7E8794` | `#8E918F` | `#80858F` | Contexto, no editable ahora |
| `faint` | `#232831` | `#262A30` | `#24262C` | Guías, bordes pasivos, pasos vacíos |
| `panel` | `#0D1118` | `#12161D` | `#0F1014` | Superficie de teclas y cápsulas |
| `rec` | `#FF3B30` | `#E63946` | `#FF2244` | Solo REC, borrar y picos |

## 3. Tipografía

Las pantallas son de 480×272 reales, así que la letra tiene que leerse a 11–15 px. Se renderiza con antialiasing (TrueType).

**Letra: Jost** (OFL), elegida por Javi el 2026-11-12, ajustada para parecerse a la pantalla del OP-XY: pesos livianos, etiquetas chicas con algo de aire entre letras y números grandes en el peso más fino. Una sola familia para todo. Teenage Engineering no publica qué letra usa el OP-XY, así que el ajuste es a ojo sobre lo que se ve en el equipo, no una copia.

| Nivel | px | Peso | Espaciado | Uso |
| :-- | :-- | :-- | :-- | :-- |
| `s` | 11 | 400 | +0.6 px | Nombre de parámetro bajo el dial |
| `m` | 15 | 400 | +0.3 px | Valor bajo el dial |
| `l` | 20 | 400 | +0.2 px | Valores del inspector |
| `mid` | 21 | 400 | +0.2 px | Instrumento, acorde, letra de pista |
| `big` | 34 | 300 | 0 | Número de paso en el inspector |
| `huge` | 62 | 300 | −1 px | Valor al tocar un encoder |

**FlipMono no se usa:** el archivo `FlipMono.ttf` de Flip Tracker+ es SF Mono de Apple, cuya licencia solo permite usarla en equipos Apple. No se puede distribuir en una Raspberry Pi.

En Rust, Jost (300 y 400, `assets/fonts/`) se incrusta con `include_bytes!` y se rasteriza con `fontdue` (MIT/Apache); cada glifo se guarda en caché y se mezcla con alfa sobre el framebuffer RGB565 (`crates/op3-gfx/src/font.rs`).

## 3b. Dónde vive cada cosa en el código

| Qué | Archivo |
| :-- | :-- |
| Temas y colores de encoder | `crates/op3-gfx/src/theme.rs` (`theme()`, `knob(i)`) |
| Letra y escala tipográfica | `crates/op3-gfx/src/font.rs` |
| Líneas, arcos, círculos y cajas con antialiasing | `crates/op3-gfx/src/paint.rs` |
| Pie de diales y overlay de toque | `crates/op3-ui/src/theme.rs` |
| Pantalla izquierda | `crates/op3-ui/src/left.rs` |
| Pantalla derecha (marco) | `crates/op3-ui/src/right.rs` |
| Dibujo de cada instrumento | `draw_ui` de cada cartridge en `crates/op3-cartridges/src/` |

El tema se elige con `op3 run -t op-1|op-1-field|op-xy` o `OP3_THEME`.

## 4. Estructura de pantalla

```
┌────────────────────────────────────────────────┐
│ ● nombre                         ● ● (páginas) │  barra superior, 32 px
│ +                                            + │
│               dibujo animado                   │  zona de dibujo
│ +                                            + │
│ ◔ valor    ◔ valor    ◔ valor    ◔ valor       │  pie, 4 × 120 px, cada columna
│   nombre     nombre     nombre     nombre      │  sobre su encoder físico
└────────────────────────────────────────────────┘
```

- **Izquierda (fija): tiempo.** Tempo, acorde del Brain, aros de polimetría (uno por pista, la elegida adelante), espejo de los 16 pads. Pie con encoders 1–4.
- **Derecha: el instrumento de la pista elegida.** Cada cartridge dibuja su ilustración animada usando los colores 5–8 para lo que esos encoders mueven. Pie con encoders 5–8.

## 5. Componentes

- **Dial:** arco de 270° y 3 px; fondo `faint`, valor en color del encoder, punto en la punta. Valor en `fg` y nombre en color del encoder al lado.
- **Pad en pantalla:** esquinas de 6 px; vacío = punto `faint`; con nota = relleno del color de pista; en edición = borde `fg` de 2 px.
- **Cápsula:** radio = alto / 2, punto LED a la izquierda, texto sin corchetes, margen mínimo de 8 px.
- **Cruces de calibración:** `+` de 3 px en las esquinas de cada vista técnica.
- **Overlay de toque:** oscurece la zona de dibujo; nombre en color del encoder, valor en `huge`, barra fina de 3 px.

## 6. Movimiento

60 FPS en el Twin; 30–60 FPS en el equipo con actualización por regiones. Los valores se interpolan en vez de saltar. Cada nota dispara un destello que decae, y los dibujos responden al nivel de audio de su pista.

## 7. Cómo se cambia este tratado

1. Se propone el cambio en el Twin.
2. Javi lo aprueba.
3. Se actualiza este documento en el mismo PR que el código.
