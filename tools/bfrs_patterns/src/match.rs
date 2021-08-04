use crate::pattern::PatternScope;

use super::pattern::Pattern;
use bfrs_common::BFCommand;
use std::collections::HashMap;

/// A match result contains the group of instructions
/// from the source that matched, along with the bindings
/// and their relative offsets.
pub struct MatchResult<'a> {
    pub commands: &'a [BFCommand],
    pub relative_offsets: HashMap<usize, HashMap<usize, isize>>,
}
/// A state machine to keep track of local state
/// in a matching context
pub struct MatchSM<'a> {
    instructions: &'a [BFCommand],
    registry: HashMap<usize, HashMap<usize, isize>>,
    offset: usize,
    last_binding: Option<usize>,
}

impl<'a> MatchSM<'a> {
    /// Obtain all possible matches from the same pattern group
    // NOTE: make pattern groups a distinction from a pattern itself.
    pub fn find_all(instructions: &'a [BFCommand], scope: &PatternScope) -> Vec<MatchResult<'a>> {
        let mut offset = 0;
        let mut result = Vec::new();
        while offset < instructions.len() {
            if let Some(res) = Self::match_single(&instructions[offset..], scope) {
                // advance by the match length.
                offset += res.commands.len();
                result.push(res);
            } else {
                offset += 1;
            }
        }
        result
    }
    /// Match a pattern through the beginning of the instructions
    pub fn match_single(
        instructions: &'a [BFCommand],
        scope: &PatternScope,
    ) -> Option<MatchResult<'a>> {
        let mut machine = Self::new(instructions);
        for pat in scope.patterns.iter() {
            if let Some(optional_action) = machine.match_target(pat) {
                if let Some(action) = optional_action {
                    machine.run_action(action);
                }
            } else {
                return None;
            }
        }
        Some(MatchResult {
            commands: if machine.offset == 0 {
                instructions
            } else {
                &instructions[..machine.offset]
            },
            relative_offsets: machine.registry,
        })
    }
    fn new(instructions: &'a [BFCommand]) -> Self {
        Self {
            instructions,
            registry: HashMap::new(),
            offset: 0,
            last_binding: None,
        }
    }
    fn run_action(&mut self, action: MatchSMAction) {
        match action {
            MatchSMAction::AdvanceInput { amount } => self.offset += amount,
            MatchSMAction::SetLastBinding { binding } => self.last_binding = Some(binding),
            MatchSMAction::NewBinding {
                offset_from_last,
                binding: name,
            } => {
                match self.last_binding {
                    None => {
                        // without a known last, the `offset_from_last` parameter
                        // is ignored and the binding is created with a single reference
                        // to itself.
                        self.registry.insert(name, {
                            let mut map = HashMap::new();
                            map.insert(name, 0);
                            map
                        });
                    }
                    Some(ref last) => {
                        // calculate the offsets from the name to the
                        // others, using its offset from the last one
                        // as the only common thing between them.
                        let this_offsets: HashMap<usize, isize> = {
                            let mut initial: HashMap<_, _> = self
                                .registry
                                .iter()
                                .map(|(other_k, other_map)| {
                                    (*other_k, other_map[last] + offset_from_last)
                                })
                                .collect();

                            initial.insert(name, 0);
                            initial
                        };

                        // now make edges in the opposite direction.
                        for (other_k, other_map) in self.registry.iter_mut() {
                            other_map.insert(name, -this_offsets[other_k]);
                        }

                        self.registry.insert(name, this_offsets);
                    }
                }
            }
            MatchSMAction::Chain(chain) => {
                // a chain of actions... it's simple to deal with.
                for action in chain {
                    self.run_action(action);
                }
            }
        }
    }

    fn match_target(&self, target: &Pattern) -> Option<Option<MatchSMAction>> {
        match target {
            Pattern::Instruction(instr) => self.match_instruction(*instr).map(Some),
            Pattern::Binding { index, strict } => {
                let binding = *index;
                let strict = *strict;
                let (offt, a) = self.calculate_offset();
                if strict && offt == 0 {
                    return None;
                }
                self.match_binding(binding, offt).map(|b| {
                    MatchSMAction::chain_optionals(a, b)
                        .map(|c| c.chain_with(MatchSMAction::SetLastBinding { binding }))
                })
            }
        }
    }

    fn match_binding(
        &self,
        binding: usize,
        offset_from_last: isize,
    ) -> Option<Option<MatchSMAction>> {
        match self.last_binding {
            Some(ref last) => {
                if !self.registry.contains_key(&binding) {
                    // a first-time binding will always match,
                    // as there is no older position to compare it to.
                    Some(Some(MatchSMAction::NewBinding {
                        offset_from_last,
                        binding,
                    }))
                } else {
                    // with a known last for reference, the offset
                    // can be checked for consistency with the previously
                    // recorded offset.
                    if self.registry[&binding][last] == offset_from_last {
                        // success, but nothing to do.
                        Some(None)
                    } else {
                        None
                    }
                }
            }
            None => {
                // with no known last, a first-time binding will always
                // match, as it's certain that there is no registry of the
                // binding itself, nor is a registry of any other binding to
                // compare its solidity.
                Some(Some(MatchSMAction::NewBinding {
                    offset_from_last,
                    binding,
                }))
            }
        }
    }

    /// literal instructions are checked directly against the source
    fn match_instruction(&self, instruction: BFCommand) -> Option<MatchSMAction> {
        self.instructions
            .get(self.offset)
            .filter(|&&i| i == instruction)
            .map(|_| MatchSMAction::AdvanceInput { amount: 1 })
    }

    /// calculate an offset from the current source. The first direction instruction
    /// dictates what direction is the offset going, and the rest
    /// will be matched according to that. If no direction instruction is recorded,
    /// the offset will be reported as zero.
    fn calculate_offset(&self) -> (isize, Option<MatchSMAction>) {
        let mut local_offset = 0;
        if let Some(direction) = self
            .instructions
            .get(self.offset)
            .filter(|i| matches!(i, BFCommand::Left | BFCommand::Right))
        {
            local_offset += 1;
            while self
                .instructions
                .get(self.offset + local_offset)
                .filter(|&i| i == direction)
                .is_some()
            {
                local_offset += 1;
            }
            (
                local_offset as isize
                    * match direction {
                        BFCommand::Left => -1,
                        BFCommand::Right => 1,
                        _ => unreachable!(),
                    },
                Some(MatchSMAction::AdvanceInput {
                    amount: local_offset,
                }),
            )
        } else {
            (0, None)
        }
    }
}

/// Models the different state mutation actions
/// that the state machine has available. Lets you control exactly when the state is modified
#[derive(Debug)]
enum MatchSMAction {
    /// a new binding was discovered.
    NewBinding {
        offset_from_last: isize,
        binding: usize,
    },
    AdvanceInput {
        amount: usize,
    },
    SetLastBinding {
        binding: usize,
    },
    Chain(Vec<MatchSMAction>),
}

impl MatchSMAction {
    #[inline]
    pub fn chain_with(self, other: Self) -> Self {
        match (self, other) {
            (Self::Chain(mut a_chain), Self::Chain(mut b_chain)) => Self::Chain({
                a_chain.reserve(b_chain.len());
                let drain = b_chain.drain(..);
                a_chain.extend(drain);
                a_chain
            }),
            (Self::Chain(mut chain), other) | (other, Self::Chain(mut chain)) => Self::Chain({
                chain.push(other);
                chain
            }),
            (a, b) => Self::Chain(vec![a, b]),
        }
    }

    #[inline]
    pub fn optional_chain(self, other: Option<Self>) -> Self {
        if let Some(other) = other {
            self.chain_with(other)
        } else {
            self
        }
    }

    #[inline]
    pub fn chain_optionals(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (Some(a), b) | (b, Some(a)) => Some(a.optional_chain(b)),
            _ => None,
        }
    }
}
