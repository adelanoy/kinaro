package com.kinaro.project

import com.kinaro.utils.TimeUnit
import kotlinx.serialization.Serializable

/** On-disk form of a delay step: waits for [duration] expressed in [unit]. */
@Serializable
data class DelayData(
    /** How long to wait, in [unit] */
    val duration: UInt,
    /** The time unit [duration] is expressed in */
    val unit: TimeUnit,
)
