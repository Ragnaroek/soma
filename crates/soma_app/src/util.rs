use std::thread;

#[cfg(feature = "web")]
/// async task spawner that works with all the different backends.
/// The task is always spawned in the current thread to avoid
/// Send issues.
pub fn spawn_async<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(any(feature = "desktop"))]
pub fn spawn_async<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("tokio runtime setup");

        rt.block_on(async move {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async move {
                    tokio::task::spawn_local(future).await.unwrap();
                })
                .await;
        });
    });
}

#[cfg(any(feature = "desktop"))]
/// task sleep that works with all the different backends
pub async fn sleep(millis: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(millis as u64)).await;
}

#[cfg(feature = "web")]
/// task sleep that works with all the different backends
pub async fn sleep(millis: u32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        let win = web_sys::window().expect("web_sys window");
        win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis as i32)
            .expect("timeout set");
    };
    let p = js_sys::Promise::new(&mut cb);
    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}
