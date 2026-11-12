# OP-3 — Para qué sirve cada botón

Parte del [Tratado de UX/UI](DESIGN.md). Reglas que mandan acá: cada botón significa una sola idea, parecida a su equivalente en el OP-XY y a lo que dice su serigrafía; Shift da la función secundaria, nunca otra idea (regla 12). Arriba de cada pantalla se ve qué hacen los botones que tiene encima y abajo qué hacen las perillas (regla 11).

Los botones y el secuenciador hacen siempre lo mismo y no crecen mucho más. Lo que crece son los instrumentos ([INSTRUMENTOS.md](INSTRUMENTOS.md)).

Cada botón de la MK3 con su función en el software Maschine (según el [manual de NI](https://docs.native-instruments.com/online-guides/maschine-mk3-manual/en/quick-reference)), su idea equivalente en el OP-XY (según la [guía de TE](https://teenage.engineering/guides/op-xy/layout)) y lo que hace en OP-3. **Estado:** ✓ hecho, ◐ parcial, ○ por hacer (ver [ROADMAP.md](ROADMAP.md)).

## 1. Modos principales (los equivalentes de instrument, auxiliary, arrange y mix del OP-XY)

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

## 2. Botones de pantalla 1–8 (encima de las pantallas)

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

## 3. Pads y sus modos

| Botón MK3 | En Maschine | En OP-XY | En OP-3 | Estado |
| :-- | :-- | :-- | :-- | :-- |
| STEP | Secuenciador por pasos | seq (pasos) | Pads = 16 pasos de la pista. Mantener un pad abre sus components | ✓ |
| KEYBOARD | Pads cromáticos | teclado | Pads = teclado cromático de la pista. Encoder 4D ◀ ▶ = octava (el − y + del OP-XY) | ○ |
| CHORDS | Modo acordes | players (chords) / brain | **Brain**: progresión de acordes. Pads = grados; mantener uno = acorde en vivo | ✓ |
| PAD MODE | Pads como sonidos | (tracks) | Pads 1–8 disparan las 8 pistas; 9–16 quedan para un cartridge de batería | ○ |
| PERFORM · Shift: elegir | Perform FX en la Smart Strip | punch-in fx | **Punch-in FX**: cada pad un efecto, la presión es la intensidad | ✓ |
| NOTE REPEAT · Shift: arp | Repetición y arpegiador | players (arp) | Repetir la nota mantenida; con Shift, arpegiador | ○ (hoy abre Punch) |
| FIXED VEL | Velocidad fija | — | Velocidad fija al grabar o tocar | ○ |

## 4. Edición

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

## 5. Transporte y tempo

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

## 6. Otros controles

| Control | En OP-3 | Estado |
| :-- | :-- | :-- |
| Perillas 1–4 | Lo que muestra el pie izquierdo (secuenciador o el módulo abierto) | ✓ |
| Perillas 5–8 | Lo que muestra el pie derecho (cartridge) | ✓ |
| Tocar una perilla | Overlay grande con su parámetro | ✓ |
| Encoder 4D: girar | Valor del elemento elegido; si no hay, tempo | ◐ |
| Encoder 4D: ▲ ▼ ◀ ▶ y push | Moverse en listas y en el inspector; push confirma | ○ |
| Smart Strip | Según PITCH, MOD o PERFORM: pitch bend, modulación o intensidad del punch-in. Hoy: filtro master | ◐ |
| PITCH / MOD / NOTES | Modo de la Smart Strip | ○ |

