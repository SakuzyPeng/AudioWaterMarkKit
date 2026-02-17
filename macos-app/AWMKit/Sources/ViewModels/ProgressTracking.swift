import AWMKit
import Foundation

// MARK: - Shared types

/// Wraps AWMAudio to satisfy Sendable without deep concurrency analysis.
struct UnsafeAudioBox: @unchecked Sendable {
    let audio: AWMAudio
}

/// Selects which phase-to-progress mapping table to use.
enum ProgressProfile {
    case embed
    case detect
}

// MARK: - Shared helpers

/// Returns a stable, canonical string key for a file URL.
func normalizedPathKey(_ url: URL) -> String {
    url.standardizedFileURL.path(percentEncoded: false)
}

/// Builds a weight map keyed by canonical path, values = file size (bytes, min 1).
func buildProgressWeights(for files: [URL]) -> [String: Double] {
    var weights: [String: Double] = [:]
    for file in files {
        let key = normalizedPathKey(file)
        let size = (try? file.resourceValues(forKeys: [.fileSizeKey]).fileSize).map(Double.init) ?? 1
        weights[key] = max(size, 1)
    }
    return weights
}

/// Starts a background task that polls the progress snapshot at 50 ms intervals
/// and calls `onProgress` with a scaled value in `[base, base + span]`.
func startProgressPolling(
    audio: UnsafeAudioBox,
    expectedOperation: AWMProgressOperationSwift,
    profile: ProgressProfile,
    base: Double,
    span: Double,
    initialProgress: Double,
    onProgress: @escaping @MainActor (Double) -> Void
) -> Task<Void, Never> {
    Task.detached(priority: .userInitiated) {
        var latest = min(max(initialProgress, 0), 1)
        var lastPhase = AWMProgressPhaseSwift.idle

        while !Task.isCancelled {
            if let snapshot = audio.audio.progressSnapshot(),
               snapshot.operation == expectedOperation {
                let mapped = mapSnapshotProgress(
                    snapshot,
                    profile: profile,
                    previous: latest,
                    lastPhase: &lastPhase
                )
                if mapped > latest {
                    latest = mapped
                    let scaled = min(1, max(0, base + mapped * span))
                    await MainActor.run {
                        onProgress(scaled)
                    }
                }
            }

            try? await Task.sleep(for: .milliseconds(50))
        }
    }
}

/// Maps a progress snapshot to a `[0, 0.98]` fraction, never decreasing from `previous`.
///
/// Capped at 0.98 so that 1.0 (100%) is reached only via the explicit `updateFileProgress(1)`
/// call after the Swift async operation has fully returned — preventing the progress bar from
/// showing 100% while post-processing (e.g. evidence recording) is still running.
func mapSnapshotProgress(
    _ snapshot: AWMProgressSnapshotSwift,
    profile: ProgressProfile,
    previous: Double,
    lastPhase: inout AWMProgressPhaseSwift
) -> Double {
    // Reserve the final 2% for confirmed Swift-side completion.
    let pollCap = 0.98

    if snapshot.state == .completed {
        return pollCap
    }

    let phaseRange = phaseInterval(for: snapshot.phase, profile: profile)
    if snapshot.determinate, snapshot.totalUnits > 0 {
        let ratio = min(max(Double(snapshot.completedUnits) / Double(snapshot.totalUnits), 0), 1)
        let mapped = phaseRange.lowerBound + (phaseRange.upperBound - phaseRange.lowerBound) * ratio
        return min(pollCap, max(previous, mapped))
    }

    let cap = max(
        phaseRange.lowerBound,
        min(pollCap, phaseRange.upperBound - max((phaseRange.upperBound - phaseRange.lowerBound) * 0.08, 0.01))
    )
    let step = snapshot.phase == lastPhase ? 0.0035 : 0.0015
    lastPhase = snapshot.phase
    let baseline = max(previous, phaseRange.lowerBound)
    return min(pollCap, min(cap, baseline + step))
}

/// Returns the `[0, 1]` sub-range assigned to a given phase under a given profile.
func phaseInterval(
    for phase: AWMProgressPhaseSwift,
    profile: ProgressProfile
) -> ClosedRange<Double> {
    switch profile {
    case .embed:
        switch phase {
        case .prepareInput, .precheck:
            return 0.00...0.15
        case .core, .routeStep, .merge:
            return 0.15...0.85
        case .evidence, .cloneCheck:
            return 0.85...0.95
        case .finalize:
            return 0.95...1.0
        case .idle:
            return 0.0...0.0
        }
    case .detect:
        switch phase {
        case .prepareInput, .precheck:
            return 0.00...0.10
        case .core, .routeStep, .merge:
            return 0.10...0.80
        case .evidence, .cloneCheck:
            return 0.80...0.95
        case .finalize:
            return 0.95...1.0
        case .idle:
            return 0.0...0.0
        }
    }
}
