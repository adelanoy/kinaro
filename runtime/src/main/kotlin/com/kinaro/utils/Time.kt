package com.kinaro.utils

import kotlinx.serialization.Serializable

/**
 * A unit of time, serialized by its variant name (e.g. `"Second"`).
 *
 * Mirrors `ki_utils::time::TimeUnit`.
 */
@Serializable
enum class TimeUnit {
    /** A thousandth of a second */
    Millisecond,

    /** A second */
    Second,

    /** Sixty seconds */
    Minute,
}
