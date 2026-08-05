use std::future;

pub(super) struct Scheduler<T: Copy> {
    interval: tokio::time::Duration,
    state: State<T>,
}

enum State<T> {
    Idle,
    Throttled {
        deadline: tokio::time::Instant,
    },
    Scheduled {
        deadline: tokio::time::Instant,
        payload: T,
    },
}

impl<T: Copy> Scheduler<T> {
    pub(super) fn new(frequency: u64) -> Self {
        Self {
            interval: tokio::time::Duration::from_micros(1000_000 / frequency),
            state: State::Idle,
        }
    }

    pub(super) fn push_task(&mut self, payload: T) -> Option<T> {
        let result;
        (self.state, result) = match &self.state {
            State::Idle => (
                State::Throttled {
                    deadline: tokio::time::Instant::now() + self.interval,
                },
                Some(payload),
            ),
            State::Throttled { deadline } => (
                State::Scheduled {
                    deadline: *deadline,
                    payload,
                },
                None,
            ),
            State::Scheduled {
                deadline,
                payload: _,
            } => (
                State::Scheduled {
                    deadline: *deadline,
                    payload,
                },
                None,
            ),
        };
        result
    }

    pub(super) async fn sleep(&mut self) -> Option<T> {
        let result;
        (self.state, result) = match &self.state {
            State::Idle => {
                future::pending::<()>().await;
                (State::Idle, None)
            }
            State::Throttled { deadline } => {
                tokio::time::sleep_until(*deadline).await;
                (State::Idle, None)
            }
            State::Scheduled { deadline, payload } => {
                tokio::time::sleep_until(*deadline).await;
                (
                    State::Throttled {
                        deadline: tokio::time::Instant::now() + self.interval,
                    },
                    Some(*payload),
                )
            }
        };
        result
    }
}
