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

---

## 1. Mapa de controles

| Control | Qué hace | Dónde se ve |
| :-- | :-- | :-- |
| Encoders 1–4 | Secuenciador: largo, división, swing, tempo (o el paso abierto en el inspector) | Pie de la pantalla izquierda |
| Encoders 5–8 | Parámetros del instrumento, página actual | Pie de la pantalla derecha |
| Botones de pantalla 5–6 | Página de parámetros del instrumento | Puntos de página arriba a la derecha |
| Grupos A–H | Elegir pista | Punto de pista en la barra superior |
| Pads | Pasos (modo step), acordes (chords), efectos (perform) | Espejo de pads en la pantalla izquierda |
| Mantener un pad | Abrir el inspector del paso | Pantalla izquierda |
| Encoder 4D | Navegar dentro del inspector; si no, tempo fino | |
| Smart Strip | Filtro master | |
| Tocar un encoder | Mostrar en grande ese parámetro | Pantalla de su lado |

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

En Rust, Jost (300 y 400) se incrusta con `include_bytes!` y se rasteriza con `fontdue` (MIT/Apache) a un atlas con alfa que se mezcla sobre el framebuffer RGB565, en lugar de las fuentes mono de `embedded-graphics`.

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
