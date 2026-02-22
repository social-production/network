use serde::{Deserialize, Serialize};

/// Every type of event that can be recorded on the Social Production network.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    // User lifecycle
    UserRegistered,
    UserEdited,
    UserUnregistered,

    // Organisation lifecycle
    OrgRegistered,
    OrgEdited,
    OrgUnregistered,

    // Project lifecycle
    ProjectPosted,
    ProjectEdited,
    ProjectStatusChanged,

    // Project updates
    ProjectUpdateAdded,
    ProjectUpdateEdited,
    ProjectUpdateDeleted,

    // Collective funding
    FundingCreated,
    FundingFunded,
    FundingDistributed,

    // Posts
    PostCreated,
    PostUpdated,
    PostDeleted,

    // Comments
    CommentAdded,

    // Events
    EventAdded,
    EventEdited,
    EventCancelled,

    // RSVP & voting
    RsvpChanged,
    VoteCast,

    // Network topology
    NodeAdded,
    NodeRemoved,
}
