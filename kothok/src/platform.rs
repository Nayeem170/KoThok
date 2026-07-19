// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::platform::Platform;
use slint::platform::WindowAdapter;
use std::rc::Rc;
use std::time::Instant;

pub struct KoboPlatform {
    pub window: Rc<MinimalSoftwareWindow>,
    pub start: Instant,
}

impl Platform for KoboPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }
    fn duration_since_start(&self) -> core::time::Duration {
        self.start.elapsed()
    }
}
