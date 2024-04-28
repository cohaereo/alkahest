// use hecs::{Component, ComponentRef};
//
// use crate::ecs::Scene;
//
// pub trait SceneExt {
//     fn query_global<'a, T: Component + ComponentRef<'a>>(&'a self) -> Option<T::Ref>;
// }
//
// impl SceneExt for Scene {
//     fn query_global<'a, T: Component + ComponentRef<'a>>(&'a self) -> Option<T::Ref> {
//         let ent = self.query::<&T>().iter().next().map(|(e, _)| e)?;
//         self.get::<T>(ent).ok()
//     }
// }
