@file:UseSerializers(UuidSerializer::class)

package com.kinaro.project

import com.kinaro.utils.UuidSerializer
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.UseSerializers
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.buildClassSerialDescriptor
import kotlinx.serialization.encoding.CompositeDecoder
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.encoding.decodeStructure
import kotlinx.serialization.encoding.encodeStructure
import java.util.UUID

/**
 * Metadata shared by every node of the test tree.
 *
 * Rust flattens a `FileTestMetadata` struct into each node; here each node holds these fields directly,
 * which gives the same JSON.
 */
sealed interface TestMetadata {
    val id: UUID
    val name: String

    /**
     * One entry per line, so version control diffs only the lines that
     * actually changed instead of one escaped multiline string. Omitted when empty.
     */
    val description: List<String>

    /** Omitted when false. */
    val disabled: Boolean
}

/** The root of the test tree. */
@Serializable
data class TestsContainer(val suites: List<TestSuite>) {
}

/**
 * A direct child of a suite, tagged by `"type"` (`"Case"` or `"Step"`).
 *
 * The tag is only written when the node is serialized as a [SuiteChild]: steps of a case have none.
 */
@Serializable
sealed interface SuiteChild : TestMetadata

@Serializable
data class TestSuite(
    override val id: UUID,
    override val name: String,
    override val description: List<String> = emptyList(),
    override val disabled: Boolean = false,
    val children: List<SuiteChild>,
) : TestMetadata

/** A test case, holding its own steps */
@Serializable
@SerialName("Case")
data class TestCase(
    override val id: UUID,
    override val name: String,
    override val description: List<String> = emptyList(),
    override val disabled: Boolean = false,
    val steps: List<TestStep>,
) : SuiteChild

/** A test step, located either in a case or directly in a suite */
@Serializable
@SerialName("Step")
data class TestStep(
    override val id: UUID,
    override val name: String,
    override val description: List<String> = emptyList(),
    override val disabled: Boolean = false,
    val data: StepData,
) : SuiteChild

/**
 * The data of a step, one variant per step kind.
 *
 * Externally tagged like a Rust enum: `{ "Delay": { ... } }`.
 */
@Serializable(with = StepDataSerializer::class)
sealed interface StepData {
    data class Rest(val data: RestData) : StepData

    data class Delay(val data: DelayData) : StepData
}

/** Serializes a [StepData] as an object holding a single key, the variant name. */
object StepDataSerializer : KSerializer<StepData> {
    private const val REST_INDEX = 0
    private const val DELAY_INDEX = 1

    override val descriptor: SerialDescriptor = buildClassSerialDescriptor("com.kinaro.project.StepData") {
        element("RestStep", RestData.serializer().descriptor, isOptional = true)
        element("Delay", DelayData.serializer().descriptor, isOptional = true)
    }

    /**
     * @throws SerializationException if the object does not hold exactly one known variant
     */
    override fun deserialize(decoder: Decoder): StepData = decoder.decodeStructure(descriptor) {
        var data: StepData? = null
        while (true) {
            val variant = when (val index = decodeElementIndex(descriptor)) {
                REST_INDEX -> StepData.Rest(decodeSerializableElement(descriptor, index, RestData.serializer()))
                DELAY_INDEX -> StepData.Delay(decodeSerializableElement(descriptor, index, DelayData.serializer()))
                CompositeDecoder.DECODE_DONE -> break
                else -> throw SerializationException("Unknown step data variant at index $index")
            }
            if (data != null) throw SerializationException("Step data must hold a single variant")
            data = variant
        }
        data ?: throw SerializationException("Step data holds no variant")
    }

    override fun serialize(encoder: Encoder, value: StepData) = encoder.encodeStructure(descriptor) {
        when (value) {
            is StepData.Rest -> encodeSerializableElement(descriptor, REST_INDEX, RestData.serializer(), value.data)
            is StepData.Delay -> encodeSerializableElement(descriptor, DELAY_INDEX, DelayData.serializer(), value.data)
        }
    }
}
