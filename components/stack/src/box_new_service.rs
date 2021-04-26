use dyn_clone::DynClone;
use crate::{layer, NewService};
use std::fmt::Formatter;

/// A middleware that wrap a enormous types limit the link-time overhead.
pub struct BoxNewService<T, S> {
    inner: Box<dyn CloneNewService<T, S> + Send + Sync>,
}

trait CloneNewService<T, S>: DynClone {
    fn inner_new_service(&mut self, t: T) -> S;
}

impl<T, N> CloneNewService<T, N::Service> for N
    where
        N: NewService<T> + Clone + Send + Sync + 'static,
        N::Service: Send + 'static,
{
    fn inner_new_service(&mut self, t: T) -> N::Service {
        self.new_service(t)
    }
}

impl<T, S> BoxNewService<T, S> {
    pub fn layer<N>() -> impl layer::Layer<N, Service=Self> + Clone + Copy
        where
            N: NewService<T, Service=S> + Clone + Send + Sync + 'static,
            S: Send + 'static,
    {
        layer::mk(Self::new)
    }

    pub fn new<N>(inner: N) -> Self
        where
            N: NewService<T, Service=S> + Clone + Send + Sync + 'static,
            S: Send + 'static,
    {
        Self {
            inner: Box::new(inner)
        }
    }
}

impl<T, S> Clone for BoxNewService<T, S> {
    fn clone(&self) -> Self {
        Self {
            inner: dyn_clone::clone_box(&*self.inner),
        }
    }
}

impl<T, S> NewService<T> for BoxNewService<T, S> {
    type Service = S;

    fn new_service(&mut self, target: T) -> Self::Service {
        self.inner.inner_new_service(target)
    }
}

impl<T, S> std::fmt::Debug for BoxNewService<T, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxNewService").finish()
    }
}