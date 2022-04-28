use thiserror::Error;

#[derive(Error, Debug)]
pub enum SvectorError<T: Ord + Clone> {
    #[error("element `{0}` already exist")]
    ElementAlreadyExist(T),
}