mod actors;
mod events;

// re-export the job coord handle as if it is the job coordinator itself.
pub use actors::coordinator::JobCoordinatorHandle as JobCoordinator;
pub use events::JobStatus;
