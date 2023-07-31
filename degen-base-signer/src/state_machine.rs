#[derive(Debug, PartialEq)]
pub enum States {
    Idle,
    DkgPublicDistribute,
    DkgPublicGather,
    DkgPrivateDistribute,
    DkgPrivateGather,
    SignGather,
    Signed,
}

pub trait StateMachine {
    fn move_to(&mut self, state: States) -> Result<(), Error>;
    fn can_move_to(&self, state: &States) -> Result<(), Error>;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Bad State Change: {0}")]
    BadStateChange(String),
}
