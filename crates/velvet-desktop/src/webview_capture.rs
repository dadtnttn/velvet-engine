//! Taking the picture of the game's page through WebView2 (`ICoreWebView2::CapturePreview`).
//!
//! WebView2 writes the picture into a memory stream, and calls back on the thread of the window,
//! which is the event loop's thread. So [`take_png`] only starts the picture, and the callback
//! reports the result later, as a loop event. Other systems have no picture support yet, and get
//! [`CaptureError::Unsupported`] at once.

use wry::WebView;

use crate::capture::CaptureError;

/// Starts a PNG picture of the page. `notify` is called exactly once with the outcome: the PNG
/// bytes, or the reason no picture was taken. `notify` may be called before this function returns
/// (when WebView2 refuses at once), so it must not assume the loop is still waiting.
#[cfg(windows)]
pub fn take_png<F>(webview: &WebView, notify: F)
where
    F: Fn(Result<Vec<u8>, CaptureError>) + Clone + 'static,
{
    use webview2_com::{
        CapturePreviewCompletedHandler,
        Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
    };
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
    use wry::WebViewExtWindows;

    // The memory stream that WebView2 writes the picture into. A null handle asks for a new one,
    // and `true` frees the memory when the last reference goes.
    // SAFETY: the null handle and `true` are the documented way to ask for a new memory stream.
    let stream = match unsafe { CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true) } {
        Ok(stream) => stream,
        Err(error) => {
            notify(Err(CaptureError::Failed(error.to_string())));
            return;
        }
    };
    let reader = stream.clone();
    let done = notify.clone();
    // WebView2 reports a failed picture as an error here (the status is already converted).
    let handler = CapturePreviewCompletedHandler::create(Box::new(move |status| {
        let result = match status {
            Ok(()) => read_stream(&reader),
            Err(error) => Err(CaptureError::Failed(error.to_string())),
        };
        done(result);
        Ok(())
    }));
    // SAFETY: the stream and the handler are alive for the call, and WebView2 keeps its own
    // references to them until the picture is finished.
    let started = unsafe {
        webview.webview().CapturePreview(
            COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
            &stream,
            &handler,
        )
    };
    if let Err(error) = started {
        notify(Err(CaptureError::Failed(error.to_string())));
    }
}

/// Starts a picture on a system without picture support. `notify` gets the answer at once.
#[cfg(not(windows))]
pub fn take_png<F>(_webview: &WebView, notify: F)
where
    F: Fn(Result<Vec<u8>, CaptureError>) + Clone + 'static,
{
    notify(Err(CaptureError::Unsupported));
}

/// Copies everything that WebView2 wrote into the memory stream.
#[cfg(windows)]
fn read_stream(stream: &windows::Win32::System::Com::IStream) -> Result<Vec<u8>, CaptureError> {
    use windows::Win32::System::Com::STREAM_SEEK_SET;

    // WebView2 wrote to the end of the stream, so go back to its start first.
    // SAFETY: `stream` is a live stream, and the call only moves its position.
    unsafe { stream.Seek(0, STREAM_SEEK_SET, None) }
        .map_err(|error| CaptureError::Failed(format!("cannot rewind the picture: {error}")))?;
    let mut bytes = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let mut read: u32 = 0;
        // SAFETY: the buffer has room for the length that is given, and `read` is a valid place
        // for the count of bytes read.
        let status = unsafe {
            stream.Read(
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                Some(&mut read as *mut u32),
            )
        };
        if status.is_err() {
            return Err(CaptureError::Failed(format!(
                "cannot read the picture: {status:?}"
            )));
        }
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read as usize]);
    }
    Ok(bytes)
}
