mod d3d;
mod dispatcher_queue;
mod snapshot;
mod tests;

use std::sync::mpsc::channel;

use bindings::Windows::Win32::System::SystemServices::DPI_AWARENESS_CONTEXT;
use bindings::Windows::Win32::System::WinRT::{RoInitialize, RO_INIT_SINGLETHREADED};
use bindings::Windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use bindings::Windows::{
    System::{DispatcherQueueController, DispatcherQueueHandler},
    UI::Composition::Core::CompositorController,
};
use d3d::{create_d3d_device, create_direct3d_device};
use tests::alpha_test;

macro_rules! run_test {
    ($test_name:ident, $($param:tt)*) => {
        {
            let success = $test_name($($param)*).await?;
            let message = match success {
                true => "PASSED",
                false => "FAILED",
            };
            println!("{}: {}", stringify!($test_name), message);
        }
    }
}

#[async_std::main]
async fn main() -> windows::Result<()> {
    // NOTE: We don't properly scale any of the UI or properly respond to DPI changes, but none of
    //       the UI is meant to be interacted with. This is just so that the tests don't get
    //       virtualized coordinates on high DPI machines.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT(-4)) };
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED)? };

    // The compositor needs a DispatcherQueue. We'll create one on a dedicated thread so that
    // we can block the main thread if we need to.
    let dispatcher_controller = DispatcherQueueController::CreateOnDedicatedThread()?;
    let compositor_queue = dispatcher_controller.DispatcherQueue()?;
    // Because the tests themselves won't be running on the compositor thread, we'll need to
    // controll when our changes are committed. Create a CompositorController so we have control
    // over calling Commit.
    let compositor_controller = {
        let (sender, receiver) = channel();
        compositor_queue.TryEnqueue(DispatcherQueueHandler::new(
            move || -> windows::Result<()> {
                let compositor_controller = CompositorController::new()?;
                sender.send(compositor_controller).unwrap();
                Ok(())
            },
        ))?;
        receiver.recv().unwrap()
    };

    // Initialize D3D
    let d3d_device = create_d3d_device()?;
    let device = create_direct3d_device(&d3d_device)?;

    // Run tests
    // TODO: Allow filters to only run certain tests
    run_test!(alpha_test, &compositor_controller, &device);

    Ok(())
}
