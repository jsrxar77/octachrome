# OP-3 — Roadmap

Sale del mapa de controles del [Tratado de UX/UI](DESIGN.md#1-mapa-de-controles): cada botón de la MK3 tiene asignada una idea, y este roadmap es el orden en que se construyen. Cada paso se prueba primero en el Twin (`docs/op3-twin.html`) y después en Rust.

## Hecho

- Motor, secuenciador con polimetría, Step Components, Brain y Punch-in FX.
- Cartridges KickSyn, FmBass y MidiOutBridge.
- Driver MK3 (pads, perillas, botones, LEDs, pantallas), Pi 5 y systemd. **Falta probarlo en una MK3 real.**
- Lenguaje visual: 8 colores de perilla, tres temas, Jost, dibujos por instrumento.
- Botones: STEP, CHORDS (Brain), PERFORM (Punch-in), grupos A–H, MUTE, ERASE + pad, PLAY, STOP, RESTART, SHIFT, pantalla 5–6.

## 1. Prueba en hardware (lo próximo)

- `op3 hw-test` en el Mac de Javi con la MK3: confirmar pads, LEDs, colores de pantalla y formato del protocolo de pantalla.

## 2. Esqueleto de navegación

- Franja superior con los rótulos de los botones de pantalla 1–8 (regla 11).
- Botones de pantalla 1–4: step, components, players, bar. 5–8: sound, shape, fx, mod.
- PAGE ◀ ▶, encoder 4D (▲ ▼ ◀ ▶, push, girar).
- SETTINGS: elegir tema desde la MK3, audio y MIDI.

## 3. Tocar y grabar

- KEYBOARD con octava en el encoder 4D.
- REC (notas y automatización), TAP + metrónomo, FIXED VEL.
- TEMPO, SWING y VOLUME con el encoder 4D.
- AUTO: automatización por perilla.

## 4. Sonido

- PLUG-IN: cambiar el cartridge de la pista.
- BROWSER: presets por cartridge.
- Módulos fx (botón 7) y mod/LFO (botón 8) por pista.
- MACRO (Auxiliar): envíos de efectos y Brain.
- NOTE REPEAT y arpegiador (players, botón 3).

## 5. Estructura

- PATTERN: 16 patrones por pista.
- SCENE y ARRANGER: escenas y canción.
- DUPLICATE, SELECT, SOLO, VARIATION, EVENTS, LOCK.
- FILE: proyectos, guardar y abrir.

## 6. Mezcla y samples

- MIXER: nivel, paneo, EQ y compresor master.
- SAMPLING y un cartridge sampler.
- Smart Strip con PITCH, MOD y NOTES.
