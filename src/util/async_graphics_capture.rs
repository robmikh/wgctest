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

pub struct AsyncGraphicsCapture {
    _item: GraphicsCaptureItem,
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    receiver: async_std::channel::Receiver<Direct3D11CaptureFrame>,
}

impl AsyncGraphicsCapture {
    pub fn new(device: &IDirect3DDevice, item: GraphicsCaptureItem) -> windows::core::Result<Self> {
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            item.Size()?,
        )?;
        let (sender, receiver) = async_std::channel::bounded(1);
        let handler = TypedEventHandler::<
        Direct3D11CaptureFramePool,
        windows::core::IInspectable,
    >::new(
        move |frame_pool, _| -> windows::core::Result<()> {
            let frame_pool = frame_pool.as_ref().unwrap();
            let frame = frame_pool.TryGetNextFrame()?;
            async_std::task::block_on(sender.send(frame)).unwrap();
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

    pub async fn get_next_frame(&self) -> windows::core::Result<Direct3D11CaptureFrame> {
        let frame = self.receiver.recv().await.unwrap();
        Ok(frame)
    }
}

impl Drop for AsyncGraphicsCapture {
    fn drop(&mut self) {
        self.session.Close().unwrap();
        self.frame_pool.Close().unwrap();
    }
}
