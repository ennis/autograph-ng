//! xxgui - Cross platform and extensible GUI
//!
//! The underlying structure is a tree of nodes.
//! Nodes have:
//! - (optional) associated state, downcastable
//! - (optional) custom draw & event handling callbacks
//! - layout (box)
//! - parent & child nodes
//!
//! Nodes with state are called 'Components' (like in react)
//!
//! This tree is updated piecewise via immediate-mode calls.
//! e.g. `update(id, new-node)`.
//! Need fast hash-map lookups.
//!
//! Golden rule: don't own (or mutably borrow for a long time) the application state.
//! -> realize that other things can update the GUI in the meantime.
//! -> issue: the state can diverge (application VS user input)
//!
