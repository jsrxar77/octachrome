# OP-3 — Instrumentos

**Los instrumentos son el centro de OP-3.** El secuenciador, el Brain, el Punch-in y los botones hacen siempre lo mismo y quedan chicos. Lo que crece es la colección de instrumentos ("cartridges"): empieza con un set chico, cualquiera puede agregar uno nuevo, y cada uno se usa igual de rápido que en un OP-XY.

Este documento manda junto con el [Tratado de UX/UI](DESIGN.md) y el [mapa de botones](BOTONES.md).

## 1. Qué es un instrumento

Un instrumento es una pieza de código que cumple el contrato `InstrumentCartridge` (`crates/op3-cartridge`). Tiene cuatro partes, siempre las mismas:

| Parte | Qué es | Límite |
| :-- | :-- | :-- |
| **Sonido** | Recibe notas, devuelve audio estéreo a 48 kHz | Tiene que entrar en el presupuesto de CPU de la Pi (sección 5) |
| **Parámetros** | 4 de sonido (botón 5, perillas 5–8) y, si hace falta, 4 de forma (botón 6) | **Máximo 8.** Si el motor tiene más, el instrumento elige los 8 que importan |
| **Dibujo** | Un gráfico animado que muestra qué hace el sonido, en la zona central de la pantalla derecha | Usa los colores de las perillas 5–8 para lo que esas perillas mueven |
| **Estado** | Guardar y cargar sus valores (presets y parameter locks) | Lista de números, versionada |

El efecto y la modulación de la pista (botones 7 y 8) no son del instrumento: son iguales para todos, así cada instrumento se mantiene simple.

## 2. Reglas para un instrumento

1. **Una idea de sonido.** Un kick, un bajo FM, un acorde de cuerdas, un resonador. No un sintetizador de 200 parámetros.
2. **Ocho perillas alcanzan.** Las 4 primeras son las que más cambian el sonido. Nombres de una palabra, en minúscula (`tune`, `decay`, `color`).
3. **El dibujo explica el sonido.** Quien no sabe síntesis tiene que ver qué cambia al girar cada perilla. Nada de texto en el dibujo.
4. **Siempre suena bien.** Cualquier combinación de las 8 perillas tiene que dar un sonido usable (como en el OP-1 y el OP-XY).
5. **Rápido.** Carga en menos de 100 ms y no reserva memoria mientras suena.

## 3. Cómo se agrega un instrumento (mecanismo de extensión)

En tres etapas, de la más simple a la más abierta:

| Etapa | Cómo | Para quién | Estado |
| :-- | :-- | :-- | :-- |
| **1. En el código** | Un crate de Rust que implementa `InstrumentCartridge` y se registra en una lista. Se compila con OP-3 | Nosotros y quien sepa Rust | ◐ (el contrato existe; falta la lista de registro y elegirlos con PLUG-IN) |
| **2. Paquete aparte** | El instrumento se compila como librería (`.so` en la Pi, `.dylib` en Mac) y se copia a una carpeta `cartridges/`. OP-3 lo carga al arrancar | Cualquiera que programe, sin tocar OP-3 | ○ |
| **3. Sin compilar** | Instrumentos descritos con un lenguaje de patches (por ejemplo Faust o un formato propio) que OP-3 convierte | Gente que diseña sonido pero no programa | ○ (a investigar) |

**Kit para dibujar:** `op3-gfx` ya trae lo necesario para los dibujos (círculos, arcos, líneas y cajas suavizados, letra Jost, colores del tema y de cada perilla). Más adelante se suma una pequeña librería de dibujos reusables: osciloscopio, envolvente, forma de onda polar, teclado, ondas. Así un instrumento nuevo se ve bien sin diseñar todo desde cero.

## 4. Set inicial y candidatos

### Ya hechos

| Instrumento | Idea | Dibujo |
| :-- | :-- | :-- |
| **KickSyn** | Kick sintético: tono, caída, golpe, saturación | Parche de tambor visto desde arriba |
| **FmBass** | Bajo FM de dos operadores | Planeta y luna en órbita, envolvente como montaña |
| **MidiOutBridge** | Manda la pista por el MIDI DIN de la MK3 a un sintetizador externo | Conector DIN y notas saliendo por el cable |

### Candidatos (motores conocidos que corren en una Pi)

| Fuente | Qué aporta | Licencia | ¿Se puede incluir? |
| :-- | :-- | :-- | :-- |
| **Plaits** (Mutable Instruments) | 16 motores de síntesis en uno: analógico virtual, FM, wavetable, acordes, voz, cuerdas, percusión. Ya pensados para 3–4 perillas | MIT | **Sí.** Ya hay ports parciales a Rust (por ejemplo el motor DX7) |
| **Braids** (Mutable Instruments) | Más de 40 osciladores de un solo control "timbre/color" | MIT | **Sí** |
| **Rings / Elements** (Mutable Instruments) | Resonadores físicos: cuerdas, campanas, tubos | MIT | **Sí** |
| **Clouds / Beads** (Mutable Instruments) | Granular sobre un sample | MIT | **Sí** (útil para el futuro sampler) |
| **Surge XT** | Sintetizador enorme con muchísimos osciladores y filtros | GPL-3.0 | **Solo si OP-3 pasa a GPL**, o si se usan sus ideas sin su código |
| **Vital** | Wavetable | GPL-3.0 | Igual que Surge |

Nota legal sobre Mutable Instruments: el código de los módulos STM32 (Plaits, Braids, Rings, Elements, Clouds, Beads) es MIT, pero el nombre "Mutable Instruments" es marca registrada y su autora pide no usar los nombres de los módulos en trabajos derivados. Los instrumentos de OP-3 basados en ese código llevan nombres propios (por ejemplo "macro", "resonator") y dicen en los créditos de dónde vienen.

### Orden propuesto

1. **macro** (a partir de Plaits): un solo instrumento que da muchos sonidos con 4 perillas (motor, armonía, timbre, morph). Es el que más rinde para empezar.
2. **resonator** (a partir de Rings): cuerdas y campanas.
3. **drums**: un instrumento de batería con 8 sonidos en los pads 9–16 (PAD MODE).
4. **sampler**: grabar con SAMPLING y tocar con el teclado.

## 5. Presupuesto en la Raspberry Pi 5

Hay que medirlo en la Pi real, pero el objetivo es: 8 pistas sonando a la vez, buffers de 128 muestras (2,7 ms) y menos del 60 % de un núcleo para todo el audio. Cada instrumento declara cuántas voces usa y OP-3 avisa si una combinación no entra.

## 6. Lo que OP-3 no va a ser

- Un DAW ni un host de plugins VST: los instrumentos son pocos parámetros y un dibujo, no paneles enormes.
- Una caja de miles de parámetros: si algo necesita más de 8 perillas, son dos instrumentos.
