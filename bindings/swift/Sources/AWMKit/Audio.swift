import Foundation
import CAWMKit

// MARK: - Channel Layout

/// Audio channel layout for multichannel processing
public enum AWMChannelLayoutSwift: Int32 {
    /// Stereo (2 channels)
    case stereo = 0
    /// 5.1 Surround (6 channels): FL FR FC LFE BL BR
    case surround51 = 1
    /// 5.1.2 (8 channels): FL FR FC LFE BL BR TFL TFR
    case surround512 = 2
    /// 7.1 Surround (8 channels): FL FR FC LFE BL BR SL SR
    case surround71 = 3
    /// 7.1.4 Atmos (12 channels)
    case surround714 = 4
    /// 9.1.6 Atmos (16 channels)
    case surround916 = 5
    /// Auto-detect from file
    case auto = -1

    /// Number of channels for this layout
    public var channels: Int {
        Int(awm_channel_layout_channels(AWMChannelLayout(rawValue: rawValue)))
    }

    /// Convert to C type
    var cLayout: AWMChannelLayout {
        AWMChannelLayout(rawValue: rawValue)
    }
}

// MARK: - Detection Results

/// Audio watermark detection result
public struct AWMDetectResultSwift {
    /// Whether watermark was found
    public let found: Bool

    /// Raw 16-byte message (if found)
    public let rawMessage: Data

    /// Detection pattern ("all" or "single")
    public let pattern: String

    /// Detection score from audiowmark (nil if unavailable)
    public let detectScore: Float?

    /// Number of bit errors
    public let bitErrors: UInt32
}

public enum AWMCloneCheckKindSwift: String {
    case exact
    case likely
    case suspect
    case unavailable
}

public struct AWMCloneCheckResultSwift {
    public let kind: AWMCloneCheckKindSwift
    public let score: Double?
    public let matchSeconds: Float?
    public let evidenceId: Int64?
    public let reason: String?
}

public struct AWMEmbedEvidenceResultSwift {
    public let snrDb: Double?
    public let snrStatus: String
    public let snrDetail: String?
}

public struct AWMAudioMediaCapabilitiesSwift {
    public let backend: String
    public let eac3Decode: Bool
    public let containerMp4: Bool
    public let containerMkv: Bool
    public let containerTs: Bool
}

public enum AWMProgressOperationSwift: Int32 {
    case none = 0
    case embed = 1
    case detect = 2
}

public enum AWMProgressPhaseSwift: Int32 {
    case idle = 0
    case prepareInput = 1
    case precheck = 2
    case core = 3
    case routeStep = 4
    case merge = 5
    case evidence = 6
    case cloneCheck = 7
    case finalize = 8
}

public enum AWMProgressStateSwift: Int32 {
    case idle = 0
    case running = 1
    case completed = 2
    case failed = 3
}

public struct AWMProgressSnapshotSwift {
    public let operation: AWMProgressOperationSwift
    public let phase: AWMProgressPhaseSwift
    public let state: AWMProgressStateSwift
    public let determinate: Bool
    public let completedUnits: UInt64
    public let totalUnits: UInt64
    public let stepIndex: UInt32
    public let stepTotal: UInt32
    public let opId: UInt64
    public let phaseLabel: String
}

/// Single channel pair detection result
public struct AWMPairResultSwift {
    /// Channel pair index (0-based)
    public let pairIndex: Int

    /// Whether watermark was found in this pair
    public let found: Bool

    /// Raw 16-byte message (if found)
    public let rawMessage: Data

    /// Number of bit errors
    public let bitErrors: UInt32
}

/// Multichannel detection result
public struct AWMMultichannelDetectResultSwift {
    /// Results for each channel pair
    public let pairs: [AWMPairResultSwift]

    /// Best result across all pairs (lowest bit errors)
    public let best: AWMDetectResultSwift?
}

/// Audio watermark operations (requires audiowmark binary)
public class AWMAudio {
    private var handle: OpaquePointer?

    /// Create Audio instance (auto-search for audiowmark)
    ///
    /// - Throws: AWMError.audiowmarkNotFound if audiowmark not in PATH
    public init() throws {
        handle = awm_audio_new()
        if handle == nil {
            throw AWMError.audiowmarkNotFound
        }
    }

    /// Create Audio instance with specific audiowmark path
    ///
    /// - Parameter binaryPath: Path to audiowmark binary
    /// - Throws: AWMError.audiowmarkNotFound if path invalid
    public init(binaryPath: String) throws {
        handle = binaryPath.withCString { ptr in
            awm_audio_new_with_binary(ptr)
        }
        if handle == nil {
            throw AWMError.audiowmarkNotFound
        }
    }

    deinit {
        if let handle = handle {
            awm_audio_free(handle)
        }
    }

    /// Set watermark strength (1-30, default: 10)
    ///
    /// Higher strength = more robust but lower audio quality
    public func setStrength(_ strength: UInt8) {
        guard let handle = handle else { return }
        awm_audio_set_strength(handle, strength)
    }

    /// Set key file for audiowmark
    public func setKeyFile(_ path: String) {
        guard let handle = handle else { return }
        path.withCString { ptr in
            awm_audio_set_key_file(handle, ptr)
        }
    }

    public func progressSnapshot() -> AWMProgressSnapshotSwift? {
        guard let handle = handle else { return nil }

        var cSnapshot = AWMProgressSnapshot()
        let code = awm_audio_progress_get(handle, &cSnapshot)
        guard code == AWM_SUCCESS.rawValue else {
            return nil
        }

        let phaseLabel = withUnsafePointer(to: cSnapshot.phase_label) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: 64) { charPtr in
                String(cString: charPtr)
            }
        }

        return AWMProgressSnapshotSwift(
            operation: AWMProgressOperationSwift(rawValue: Int32(cSnapshot.operation.rawValue)) ?? .none,
            phase: AWMProgressPhaseSwift(rawValue: Int32(cSnapshot.phase.rawValue)) ?? .idle,
            state: AWMProgressStateSwift(rawValue: Int32(cSnapshot.state.rawValue)) ?? .idle,
            determinate: cSnapshot.determinate,
            completedUnits: cSnapshot.completed_units,
            totalUnits: cSnapshot.total_units,
            stepIndex: cSnapshot.step_index,
            stepTotal: cSnapshot.step_total,
            opId: cSnapshot.op_id,
            phaseLabel: phaseLabel
        )
    }

    public func clearProgress() {
        guard let handle = handle else { return }
        awm_audio_progress_clear(handle)
    }

    /// Check if audiowmark is available
    public var isAvailable: Bool {
        guard let handle = handle else { return false }
        return awm_audio_is_available(handle)
    }

    /// Query media decode capabilities.
    public func mediaCapabilities() throws -> AWMAudioMediaCapabilitiesSwift {
        guard let handle = handle else {
            throw AWMError.audiowmarkNotFound
        }

        var cCaps = AWMAudioMediaCapabilities()
        let code = awm_audio_media_capabilities(handle, &cCaps)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }

        let backend = withUnsafePointer(to: cCaps.backend) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: 16) { cPtr in
                String(cString: cPtr)
            }
        }

        return AWMAudioMediaCapabilitiesSwift(
            backend: backend,
            eac3Decode: cCaps.eac3_decode,
            containerMp4: cCaps.container_mp4,
            containerMkv: cCaps.container_mkv,
            containerTs: cCaps.container_ts
        )
    }

    /// Embed watermark into audio file
    ///
    /// - Parameters:
    ///   - input: Input audio file URL
    ///   - output: Output audio file URL
    ///   - message: 16-byte message to embed
    /// - Throws: AWMError on failure
    public func embed(input: URL, output: URL, message: Data) throws {
        guard message.count == 16 else {
            throw AWMError.invalidMessageLength(message.count)
        }

        guard let handle = handle else {
            throw AWMError.audiowmarkNotFound
        }

        let result = input.path.withCString { inputPtr in
            output.path.withCString { outputPtr in
                message.withUnsafeBytes { msgPtr in
                    awm_audio_embed(
                        handle,
                        inputPtr,
                        outputPtr,
                        msgPtr.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    )
                }
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }
    }

    /// Convenience: Encode tag and embed watermark
    ///
    /// - Parameters:
    ///   - input: Input audio file URL
    ///   - output: Output audio file URL
    ///   - tag: Tag to embed
    ///   - key: HMAC key
    /// - Returns: The encoded 16-byte message
    /// - Throws: AWMError on failure
    @discardableResult
    public func embed(input: URL, output: URL, tag: AWMTag, key: Data) throws -> Data {
        let message = try AWMMessage.encode(tag: tag, key: key)
        try embed(input: input, output: output, message: message)
        return message
    }

    /// Detect watermark from audio file
    ///
    /// - Parameter input: Audio file URL
    /// - Returns: Detection result (nil if no watermark found)
    /// - Throws: AWMError on failure (not for "no watermark found")
    public func detect(input: URL) throws -> AWMDetectResultSwift? {
        guard let handle = handle else {
            throw AWMError.audiowmarkNotFound
        }

        var cResult = AWMDetectResult()

        let result = input.path.withCString { inputPtr in
            awm_audio_detect(handle, inputPtr, &cResult)
        }

        if result == AWM_ERROR_NO_WATERMARK_FOUND.rawValue {
            return nil
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        // Convert raw_message to Data
        let rawMessage = withUnsafeBytes(of: cResult.raw_message) { ptr in
            Data(ptr)
        }

        // Convert pattern to String
        let pattern = withUnsafePointer(to: cResult.pattern) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: 16) { charPtr in
                String(cString: charPtr)
            }
        }

        return AWMDetectResultSwift(
            found: cResult.found,
            rawMessage: rawMessage,
            pattern: pattern,
            detectScore: cResult.has_detect_score ? cResult.detect_score : nil,
            bitErrors: cResult.bit_errors
        )
    }

    /// Evaluate clone check for decoded identity/key slot
    public func cloneCheck(input: URL, identity: String, keySlot: UInt8) throws -> AWMCloneCheckResultSwift {
        var cResult = AWMCloneCheckResult()

        let result = input.path.withCString { inputPtr in
            identity.withCString { identityPtr in
                awm_clone_check_for_file(inputPtr, identityPtr, keySlot, &cResult)
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        let kind: AWMCloneCheckKindSwift
        switch cResult.kind {
        case AWM_CLONE_CHECK_EXACT:
            kind = .exact
        case AWM_CLONE_CHECK_LIKELY:
            kind = .likely
        case AWM_CLONE_CHECK_SUSPECT:
            kind = .suspect
        default:
            kind = .unavailable
        }

        let reason = withUnsafePointer(to: cResult.reason) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: 128) { charPtr in
                String(cString: charPtr)
            }
        }

        return AWMCloneCheckResultSwift(
            kind: kind,
            score: cResult.has_score ? cResult.score : nil,
            matchSeconds: cResult.has_match_seconds ? cResult.match_seconds : nil,
            evidenceId: cResult.has_evidence_id ? cResult.evidence_id : nil,
            reason: reason.isEmpty ? nil : reason
        )
    }

    /// Build and record audio evidence for embedded output file
    public func recordEvidence(file: URL, rawMessage: Data, key: Data, isForcedEmbed: Bool = false) throws {
        guard rawMessage.count == 16 else {
            throw AWMError.invalidMessageLength(rawMessage.count)
        }

        let result = file.path.withCString { filePathPtr in
            rawMessage.withUnsafeBytes { rawPtr in
                key.withUnsafeBytes { keyPtr in
                    awm_evidence_record_file_ex(
                        filePathPtr,
                        rawPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        key.count,
                        isForcedEmbed
                    )
                }
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }
    }

    /// Build and record audio evidence for embedded output file, and return SNR analysis.
    public func recordEmbedEvidence(
        input: URL,
        output: URL,
        rawMessage: Data,
        key: Data,
        isForcedEmbed: Bool = false
    ) throws -> AWMEmbedEvidenceResultSwift {
        guard rawMessage.count == 16 else {
            throw AWMError.invalidMessageLength(rawMessage.count)
        }

        var cResult = AWMEmbedEvidenceResult()
        let result = input.path.withCString { inputPathPtr in
            output.path.withCString { outputPathPtr in
                rawMessage.withUnsafeBytes { rawPtr in
                    key.withUnsafeBytes { keyPtr in
                        awm_evidence_record_embed_file_ex(
                            inputPathPtr,
                            outputPathPtr,
                            rawPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                            keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                            key.count,
                            isForcedEmbed,
                            &cResult
                        )
                    }
                }
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        let snrStatus = withUnsafePointer(to: cResult.snr_status) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: 16) { charPtr in
                String(cString: charPtr)
            }
        }
        let snrDetailRaw = withUnsafePointer(to: cResult.snr_detail) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: 128) { charPtr in
                String(cString: charPtr)
            }
        }

        return AWMEmbedEvidenceResultSwift(
            snrDb: cResult.has_snr_db ? cResult.snr_db : nil,
            snrStatus: snrStatus.isEmpty ? "unavailable" : snrStatus,
            snrDetail: snrDetailRaw.isEmpty ? nil : snrDetailRaw
        )
    }

    /// Convenience: Detect and decode watermark
    ///
    /// - Parameters:
    ///   - input: Audio file URL
    ///   - key: HMAC key for verification
    /// - Returns: Decoded message result (nil if no watermark or invalid)
    /// - Throws: AWMError on failure
    public func detectAndDecode(input: URL, key: Data) throws -> AWMMessageResult? {
        guard let detectResult = try detect(input: input) else {
            return nil
        }

        do {
            return try AWMMessage.decode(detectResult.rawMessage, key: key)
        } catch AWMError.hmacMismatch {
            // Watermark found but HMAC invalid
            return nil
        }
    }

    // MARK: - Multichannel Operations

    /// Embed watermark into multichannel audio file
    ///
    /// - Parameters:
    ///   - input: Input audio file URL
    ///   - output: Output audio file URL
    ///   - message: 16-byte message to embed
    ///   - layout: Channel layout (nil for auto-detect)
    /// - Throws: AWMError on failure
    public func embedMultichannel(input: URL, output: URL, message: Data, layout: AWMChannelLayoutSwift? = nil) throws {
        guard message.count == 16 else {
            throw AWMError.invalidMessageLength(message.count)
        }

        guard let handle = handle else {
            throw AWMError.audiowmarkNotFound
        }

        let cLayout = layout?.cLayout ?? AWMChannelLayout(rawValue: AWMChannelLayoutSwift.auto.rawValue)

        let result = input.path.withCString { inputPtr in
            output.path.withCString { outputPtr in
                message.withUnsafeBytes { msgPtr in
                    awm_audio_embed_multichannel(
                        handle,
                        inputPtr,
                        outputPtr,
                        msgPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        cLayout
                    )
                }
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }
    }

    /// Convenience: Encode tag and embed multichannel watermark
    ///
    /// - Parameters:
    ///   - input: Input audio file URL
    ///   - output: Output audio file URL
    ///   - tag: Tag to embed
    ///   - key: HMAC key
    ///   - layout: Channel layout (nil for auto-detect)
    /// - Returns: The encoded 16-byte message
    /// - Throws: AWMError on failure
    @discardableResult
    public func embedMultichannel(input: URL, output: URL, tag: AWMTag, key: Data, layout: AWMChannelLayoutSwift? = nil) throws -> Data {
        let message = try AWMMessage.encode(tag: tag, key: key)
        try embedMultichannel(input: input, output: output, message: message, layout: layout)
        return message
    }

    /// Detect watermark from multichannel audio file
    ///
    /// - Parameters:
    ///   - input: Audio file URL
    ///   - layout: Channel layout (nil for auto-detect)
    /// - Returns: Multichannel detection result
    /// - Throws: AWMError on failure
    public func detectMultichannel(input: URL, layout: AWMChannelLayoutSwift? = nil) throws -> AWMMultichannelDetectResultSwift {
        guard let handle = handle else {
            throw AWMError.audiowmarkNotFound
        }

        var cResult = AWMMultichannelDetectResult()
        let cLayout = layout?.cLayout ?? AWMChannelLayout(rawValue: AWMChannelLayoutSwift.auto.rawValue)

        let result = input.path.withCString { inputPtr in
            awm_audio_detect_multichannel(handle, inputPtr, cLayout, &cResult)
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        // Convert pair results (C array is imported as tuple in Swift)
        var pairs: [AWMPairResultSwift] = []
        withUnsafePointer(to: &cResult.pairs) { tuplePtr in
            tuplePtr.withMemoryRebound(to: AWMPairResult.self, capacity: 8) { arrayPtr in
                for i in 0..<Int(cResult.pair_count) {
                    let pair = arrayPtr[i]
                    let rawMessage = withUnsafeBytes(of: pair.raw_message) { Data($0) }
                    pairs.append(AWMPairResultSwift(
                        pairIndex: Int(pair.pair_index),
                        found: pair.found,
                        rawMessage: rawMessage,
                        bitErrors: pair.bit_errors
                    ))
                }
            }
        }

        // Convert best result
        var best: AWMDetectResultSwift? = nil
        if cResult.has_best {
            let bestRawMessage = withUnsafeBytes(of: cResult.best_raw_message) { Data($0) }
            let bestPattern = withUnsafePointer(to: cResult.best_pattern) { ptr in
                ptr.withMemoryRebound(to: CChar.self, capacity: 16) { charPtr in
                    String(cString: charPtr)
                }
            }
            best = AWMDetectResultSwift(
                found: true,
                rawMessage: bestRawMessage,
                pattern: bestPattern,
                detectScore: cResult.has_best_detect_score ? cResult.best_detect_score : nil,
                bitErrors: cResult.best_bit_errors
            )
        }

        return AWMMultichannelDetectResultSwift(pairs: pairs, best: best)
    }

    /// Convenience: Detect multichannel and decode watermark
    ///
    /// - Parameters:
    ///   - input: Audio file URL
    ///   - key: HMAC key for verification
    ///   - layout: Channel layout (nil for auto-detect)
    /// - Returns: Decoded message result (nil if no watermark or invalid)
    /// - Throws: AWMError on failure
    public func detectMultichannelAndDecode(input: URL, key: Data, layout: AWMChannelLayoutSwift? = nil) throws -> AWMMessageResult? {
        let mcResult = try detectMultichannel(input: input, layout: layout)

        guard let best = mcResult.best else {
            return nil
        }

        do {
            return try AWMMessage.decode(best.rawMessage, key: key)
        } catch AWMError.hmacMismatch {
            return nil
        }
    }
}
