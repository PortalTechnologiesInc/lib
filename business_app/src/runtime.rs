use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use tokio::sync::oneshot;

pub struct BindingsRuntime {
    tasks: Arc<Mutex<VecDeque<Pin<Box<dyn Future<Output = ()> + Send>>>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl BindingsRuntime {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    pub fn add_task<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        task: F,
    ) -> oneshot::Receiver<T> {
        let (sender, receiver) = oneshot::channel();
        self.tasks.lock().unwrap().push_back(Box::pin(async move {
            let result = task.await;
            let _ = sender.send(result);
        }));

        if let Some(waker) = self.waker.lock().unwrap().take() {
            waker.wake();
        }

        receiver
    }

    pub fn run(&self) -> impl Future<Output = ()> + Send {
        RuntimePoller {
            tasks: self.tasks.clone(),
            waker: self.waker.clone(),
        }
    }

    pub fn shutdown(&self) {
        self.tasks.lock().unwrap().clear();
        self.waker.lock().unwrap().take();
    }
}

pub struct RuntimePoller {
    tasks: Arc<Mutex<VecDeque<Pin<Box<dyn Future<Output = ()> + Send>>>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl Future for RuntimePoller {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.waker.lock().unwrap().replace(cx.waker().clone());

        let mut to_remove = Vec::new();
        let mut tasks = self.tasks.lock().unwrap();

        for (i, task) in tasks.iter_mut().enumerate() {
            match Future::poll(task.as_mut(), cx) {
                Poll::Ready(_) => {
                    log::trace!("Task {i} completed");
                    to_remove.push(i);
                }
                Poll::Pending => {
                    continue;
                }
            }
        }

        for i in to_remove.iter() {
            tasks.remove(*i);
        }

        Poll::Pending
    }
}
