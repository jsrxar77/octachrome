# Vendored DSP code

Copies of the files the Octachrome "macro" and "drums" instruments need, unmodified except for one change: the random generator state in `stmlib/utils/random.{h,cc}` is `thread_local` (it was a plain static), so that a run on one thread is repeatable whatever other threads do (the test bench renders audio on the test thread):

| Directory | Origin | Commit | Licence |
| :-- | :-- | :-- | :-- |
| `plaits/` | https://github.com/pichenettes/eurorack (`plaits/dsp`, `plaits/resources.*`, `plaits/user_data.h`) | f4cfa76 | MIT (notice in every file header; the repository README states "Code (STM32F projects): MIT license") |
| `stmlib/` | https://github.com/pichenettes/stmlib | d18def8 | MIT (`LICENSE-stmlib`) |

Copyright Emilie Gillet. MIT notice, as in the file headers:

> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in
> all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
> THE SOFTWARE.

"Mutable Instruments" is a registered trademark. Following the author's
guidelines for derivative works, Octachrome does not use the module's name for the
instrument built on this code: it is called **macro**.

The code is compiled in its desktop configuration (`-DTEST`), at 48 kHz.
