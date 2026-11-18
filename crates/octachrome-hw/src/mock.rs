//! A scriptable, recording [`ControlSurface`] for tests and the simulator.

use crate::{ControlEvent, ControlSurface, FrameBuffer480x272, HwResult, LedFrame, Rect, Screen};
use std::collections::VecDeque;

#[derive(Default)]
pub struct MockSurface {
    /// Events returned by the next `poll`.
    pub input: VecDeque<ControlEvent>,
    pub leds: LedFrame,
    pub led_writes: usize,
    /// What each screen currently shows (only dirty rects are copied, exactly
    /// as the device would update).
    pub screens: [FrameBuffer480x272; 2],
    pub screen_writes: usize,
    pub pixels_sent: u64,
}

impl MockSurface {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, e: ControlEvent) {
        self.input.push_back(e);
    }
}

impl ControlSurface for MockSurface {
    fn name(&self) -> &str {
        "Mock MK3"
    }

    fn poll(&mut self, out: &mut Vec<ControlEvent>) -> HwResult<()> {
        out.extend(self.input.drain(..));
        Ok(())
    }

    fn write_leds(&mut self, leds: &LedFrame) -> HwResult<()> {
        self.leds = *leds;
        self.led_writes += 1;
        Ok(())
    }

    fn write_screen(
        &mut self,
        screen: Screen,
        fb: &FrameBuffer480x272,
        rects: &[Rect],
    ) -> HwResult<()> {
        let dst = &mut self.screens[screen as usize];
        for r in rects {
            for y in r.y as usize..r.bottom() as usize {
                let s = y * FrameBuffer480x272::WIDTH;
                let (a, b) = (s + r.x as usize, s + r.right() as usize);
                dst.pixels_mut()[a..b].copy_from_slice(&fb.pixels()[a..b]);
            }
            self.pixels_sent += r.area() as u64;
        }
        self.screen_writes += 1;
        Ok(())
    }
}
