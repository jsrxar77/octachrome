# Security

Octachrome is a groovebox that runs on a Raspberry Pi or a Mac, reads a USB controller and a MIDI port, and reads and writes project, preset and sample files in `~/.config/octachrome`. It opens no network ports. The places where a flaw could matter are:

* **Files it reads:** project files (`.ocp`), presets, WAV samples and the settings file. A crafted file should be refused, never crash the program or run code.
* **Devices it reads:** USB HID from the MK3 and MIDI input.
* **The site and the twin:** static HTML and JavaScript (`guide/`, `twin/`).

## Reporting

Please report a vulnerability **privately**: on the repository, *Security → Report a vulnerability* (GitHub private vulnerability reporting). Say what you found, how to reproduce it and, if you can, which version (the commit). Do not open a public issue with the details.

You will get an answer as soon as the maintainer can; this is a small project. Fixes go to `main`; there are no versioned releases to backport to yet.
