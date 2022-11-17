mod tests;
#[macro_use]
mod util;

use std::sync::mpsc::channel;

use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::{
    System::{DispatcherQueueController, DispatcherQueueHandler},
    UI::Composition::Core::CompositorController,
};

use crate::tests::{alpha_test, basic_window_test, fullscreen_transition_test};
use crate::util::d3d::{create_d3d_device, create_direct3d_device};

#[async_std::main]
async fn main() -> windows::core::Result<()> {
    // NOTE: We don't properly scale any of the UI or properly respond to DPI changes, but none of
    //       the UI is meant to be interacted with. This is just so that the tests don't get
    //       virtualized coordinates on high DPI machines.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };

    // The compositor needs a DispatcherQueue. We'll create one on a dedicated thread so that
    // we can block the main thread if we need to.
    let dispatcher_controller = DispatcherQueueController::CreateOnDedicatedThread()?;
    let compositor_queue = dispatcher_controller.DispatcherQueue()?;
    // Because the tests themselves won't be running on the compositor thread, we'll need to
    // controll when our changes are committed. Create a CompositorController so we have control
    // over calling Commit.
    let compositor_controller = {
        let (sender, receiver) = channel();
        let handler = DispatcherQueueHandler::new(
            move || -> windows::core::Result<()> {
                let compositor_controller = CompositorController::new()?;
                sender.send(compositor_controller).unwrap();
                Ok(())
            },
        );
        compositor_queue.TryEnqueue(&handler)?;
        receiver.recv().unwrap()
    };

    // Initialize D3D
    let d3d_device = create_d3d_device()?;
    let device = create_direct3d_device(&d3d_device)?;

    // Run tests
    // TODO: Allow filters to only run certain tests
    run_test!(alpha_test, &compositor_controller, &device);
    run_test!(
        basic_window_test,
        &compositor_queue,
        &compositor_controller,
        &device
    );
    run_test!(fullscreen_transition_test, &compositor_queue, &device);

    Ok(())
}
