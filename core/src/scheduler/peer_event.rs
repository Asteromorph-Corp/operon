/// `IndividualScheduler`-`IndividualScheduler` communication events.
///
/// These are used for communication between individual schedulers,
/// where each scheduler should modify its tickets based on the events.
#[derive(Debug, Clone)]
enum PeerEvent {
    /// A job was run and finished.
    Job(job::Job),
    /// A resolution was made known.
    Resolution(masked_dimension::Resolution),
}
