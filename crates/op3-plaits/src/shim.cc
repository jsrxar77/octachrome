// C ABI around the vendored oscillator voice, for op3-plaits (Rust).
#include <new>
#include "plaits/dsp/voice.h"

namespace {
const size_t kRam = 32 * 1024;
struct Handle {
  plaits::Voice voice;
  char ram[kRam];
};
}

extern "C" {

struct Op3MacroPatch {
  float note;
  float harmonics;
  float timbre;
  float morph;
  int engine;
  float decay;
  float lpg_colour;
  float trigger;
};

void* op3_macro_new() {
  Handle* h = new (std::nothrow) Handle;
  if (!h) return nullptr;
  stmlib::BufferAllocator allocator(h->ram, kRam);
  h->voice.Init(&allocator);
  return h;
}

void op3_macro_free(void* p) { delete static_cast<Handle*>(p); }

// Renders `size` (<= 24) samples into out/aux as floats in -1..1.
void op3_macro_render(void* p, const Op3MacroPatch* in, float* out, float* aux, size_t size) {
  Handle* h = static_cast<Handle*>(p);
  plaits::Patch patch = {};
  patch.note = in->note;
  patch.harmonics = in->harmonics;
  patch.timbre = in->timbre;
  patch.morph = in->morph;
  patch.engine = in->engine;
  patch.decay = in->decay;
  patch.lpg_colour = in->lpg_colour;
  plaits::Modulations mod = {};
  mod.trigger = in->trigger;
  mod.trigger_patched = true;
  plaits::Voice::Frame frames[plaits::kMaxBlockSize];
  if (size > plaits::kMaxBlockSize) size = plaits::kMaxBlockSize;
  h->voice.Render(patch, mod, frames, size);
  for (size_t i = 0; i < size; ++i) {
    out[i] = frames[i].out * (1.0f / 32768.0f);
    aux[i] = frames[i].aux * (1.0f / 32768.0f);
  }
}

int op3_macro_active_engine(const void* p) {
  return static_cast<const Handle*>(p)->voice.active_engine();
}

}
