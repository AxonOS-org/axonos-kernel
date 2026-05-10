# Capability-Based Application Isolation

## Threat Model

- **A1**: Malicious installed application attempting to exfiltrate raw EEG data
- **A2**: Compromised application exploiting memory-safety vulnerability in application layer
- **A3**: Network adversary intercepting intent observations in transit

## Capability Model (Definition 8.1)

A capability κ = (T, r) is a typed permission token where:
- T ∈ T is the event type
- r ∈ R+ is the maximum delivery rate (Hz)

The kernel maintains a static catalogue K ⊂ T × R+.
An application's manifest M ⊆ K is verified at install time.

## Prohibited Types (Definition 8.2)

Types T ∉ π₁(K) (permanently absent from catalogue):
- RawEEG
- ContinuousEmotion
- CognitiveProfile
- Reidentification

## Permitted Catalogue (Table 9)

| Capability | Payload | Max Rate |
|-----------|---------|----------|
| Navigation | {Left, Right, Up, Down, Idle} | 50 Hz |
| WorkloadAdvisory | {Low, Medium, High} | 1 Hz |
| SessionQuality | {Good, Degraded, Lost} | 2 Hz |
| ArtifactEvents | {Eye, Muscle, Motion, Electrode} | 10 Hz |

## Theorem 8.3 (Structural Data Minimisation)

Under the AxonOS manifest system, for any application A with manifest M ⊆ K, no event of a prohibited type can be delivered to A.

**Proof**: The kernel's event-delivery function dispatch(T) = ∅ for all prohibited T, since (T, r) ∉ M_A for all r and all applications A.

Furthermore, the signal path computes the event type before dispatch: raw EEG is processed by the pipeline (FIR, CSP, LDA), producing a discrete-valued event in the permitted catalogue. The raw EEG tensor is never serialised into any IPC message.

## Information-Theoretic Privacy Bound (Theorem 9.1)

I(X; Y) ≤ H(Y) ≤ Σ_κ r_κ · log₂|P(κ)| bits/s

For default manifest with all 4 capabilities:
I(X; Y) ≤ 50·log₂5 + 1·log₂3 + 2·log₂3 + 10·log₂4 = 140.85 bits/s

Context: raw ADC output is 48,000 bits/s. The 140.85 bits/s figure is a maximum-entropy upper bound.

## Min-Entropy Residual (Theorem 9.3)

H_∞(X|Y) ≥ H_∞(X) - log₂|Y| = H_∞(X) - 7.49 bits

Observing the full capability event stream reduces adversary's uncertainty about underlying EEG by at most 7.49 bits — operationally negligible.
