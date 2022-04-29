use thiserror::Error;

#[derive(Error, Debug)]
pub enum SortedContainersError<T: Ord + Clone> {
    #[error("element `{0}` already exist")]
    ElementAlreadyExist(T),
    #[error("element `{0}` not found")]
    ElementNotFound(T),
}
