use std::sync::mpsc::{channel, Receiver};

use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
};

pub struct GraphicsCapture {
    _item: GraphicsCaptureItem,
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    receiver: Receiver<Direct3D11CaptureFrame>,
}

impl GraphicsCapture {
    pub fn new(device: &IDirect3DDevice, item: GraphicsCaptureItem) -> windows::core::Result<Self> {
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            item.Size()?,
        )?;
        let (sender, receiver) = channel();
        let handler =
            TypedEventHandler::<Direct3D11CaptureFramePool, windows::core::IInspectable>::new(
                move |frame_pool, _| -> windows::core::Result<()> {
                    let frame_pool = frame_pool.as_ref().unwrap();
                    let frame = frame_pool.TryGetNextFrame()?;
                    sender.send(frame).unwrap();
                    Ok(())
                },
            );
        frame_pool.FrameArrived(&handler)?;
        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;
        Ok(Self {
            _item: item,
            frame_pool,
            session,
            receiver,
        })
    }

    pub fn get_next_frame(&self) -> windows::core::Result<Direct3D11CaptureFrame> {
        let frame = self.receiver.recv().unwrap();
        Ok(frame)
    }
}

impl Drop for GraphicsCapture {
    fn drop(&mut self) {
        self.session.Close().unwrap();
        self.frame_pool.Close().unwrap();
    }
}
