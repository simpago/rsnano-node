use std::any::Any;

use crate::utils::{Deserialize, Serialize, StreamAdapter};

pub trait DbIterator<K, V> {
    fn is_end(&self) -> bool;
    fn current(&self) -> Option<(&K, &V)>;
    fn next(&mut self);
    fn eq(&self, other: &dyn DbIterator<K, V>) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait DbIteratorImpl: PartialEq {
    fn current(&self) -> Option<(&[u8], &[u8])>;
    fn next(&mut self);
}

pub struct BinaryDbIterator<K, V, I>
where
    K: Serialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
    I: DbIteratorImpl + PartialEq,
{
    iterator_impl: Option<I>,
    current: Option<(K, V)>,
}

impl<K, V, I> PartialEq for BinaryDbIterator<K, V, I>
where
    K: Serialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
    I: DbIteratorImpl + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.iterator_impl == other.iterator_impl
    }
}

impl<K, V, I> BinaryDbIterator<K, V, I>
where
    K: Serialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
    I: DbIteratorImpl + PartialEq,
{
    pub fn new(iterator_impl: I) -> Self {
        let mut result = Self {
            iterator_impl: Some(iterator_impl),
            current: None,
        };
        result.load_current();
        result
    }

    fn load_current(&mut self) {
        self.current = match self.iterator_impl.as_ref().unwrap().current() {
            Some((k, v)) => {
                if k.len() < K::serialized_size() {
                    None
                } else {
                    let key = K::deserialize(&mut StreamAdapter::new(k)).unwrap();
                    let value = V::deserialize(&mut StreamAdapter::new(v)).unwrap();
                    Some((key, value))
                }
            }
            None => None,
        };
    }

    pub fn take_impl(&mut self) -> I {
        self.iterator_impl.take().unwrap()
    }
}

impl<K, V, I> DbIterator<K, V> for BinaryDbIterator<K, V, I>
where
    K: Serialize + Deserialize<Target = K> + 'static,
    V: Deserialize<Target = V> + 'static,
    I: DbIteratorImpl + PartialEq + 'static,
{
    fn is_end(&self) -> bool {
        self.iterator_impl.as_ref().unwrap().current().is_none()
    }

    fn current(&self) -> Option<(&K, &V)> {
        self.current.as_ref().map(|(k, v)| (k, v))
    }

    fn next(&mut self) {
        self.iterator_impl.as_mut().unwrap().next();
        self.load_current();
    }

    fn eq(&self, other: &dyn DbIterator<K, V>) -> bool {
        let other = other
            .as_any()
            .downcast_ref::<BinaryDbIterator<K, V, I>>()
            .unwrap();
        self.iterator_impl.eq(&other.iterator_impl)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
