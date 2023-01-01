pub trait Loggable<T, E> {
    fn log_error<C>(self, context: C) -> Result<T, E>
    where
        C: std::fmt::Display + Send + Sync + 'static;
}


// impl<T, E> Loggable<T, E> for Result<T, E>
// where
//     E: std::error::Error + Send + Sync + 'static {

//     fn log_error<C: std::fmt::Display + Send + Sync + 'static>(self, context: C) -> Result<T, E> {
//         self
//             .map_err(|e| {
//                 tracing::error!("{} {:#?}", context, e);

//                 e
//             })
//     }
// }


pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;


impl<T> Loggable<T, BoxedError> for Result<T, BoxedError> {
    fn log_error<C: std::fmt::Display + Send + Sync + 'static>(self, context: C) -> Result<T, BoxedError> {
        self
            .map_err(|e| {
                tracing::error!("{} {:#?}", context, e);

                e
            })
    }
}
