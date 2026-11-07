//! Score handler, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_handler/np_handler_score.cpp`.

use std::collections::HashMap;

/// The score request context the handler fulfils.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScoreRequest {
    pub request_id: i32,
    pub title_ctx_id: i32,
    pub user_id: i32,
    pub board_id: u32,
    pub value: i64,
    pub completed: bool,
    pub result: i32,
}

/// `np_handler_score.cpp` state.
#[derive(Debug, Default)]
pub struct NpHandlerScore {
    pub requests: HashMap<i32, ScoreRequest>,
    pub scores: HashMap<(u32, i32), i64>,
}

impl NpHandlerScore {
    pub fn submit(&mut self, request: ScoreRequest) {
        self.requests.insert(request.request_id, request);
    }

    pub fn complete(&mut self, request_id: i32, result: i32) -> bool {
        match self.requests.get_mut(&request_id) {
            Some(request) => {
                request.completed = true;
                request.result = result;
                if result == 0 {
                    self.scores
                        .insert((request.board_id, request.user_id), request.value);
                }
                true
            }
            None => false,
        }
    }

    pub fn rank(&self, board_id: u32, user_id: i32) -> Option<u32> {
        let value = *self.scores.get(&(board_id, user_id))?;
        let better = self
            .scores
            .iter()
            .filter(|((board, _), score)| *board == board_id && **score > value)
            .count();
        Some(better as u32 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_handler_completes_and_ranks() {
        let mut handler = NpHandlerScore::default();
        handler.submit(ScoreRequest {
            request_id: 1,
            board_id: 7,
            user_id: 1,
            value: 10,
            ..ScoreRequest::default()
        });
        assert!(handler.complete(1, 0));
        assert_eq!(handler.rank(7, 1), Some(1));
        assert!(!handler.complete(2, 0));
    }
}
