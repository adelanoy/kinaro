package com.kinaro.utils

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeParseException
import java.util.UUID

/**
 * Serializes a [UUID] as its hyphenated string form, as the Rust `uuid` crate does.
 */
object UuidSerializer : KSerializer<UUID> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("java.util.UUID", PrimitiveKind.STRING)

    /**
     * @throws SerializationException if the string is not a valid UUID
     */
    override fun deserialize(decoder: Decoder): UUID {
        val value = decoder.decodeString()
        return try {
            UUID.fromString(value)
        } catch (err: IllegalArgumentException) {
            throw SerializationException("Invalid UUID: $value", err)
        }
    }

    override fun serialize(encoder: Encoder, value: UUID) = encoder.encodeString(value.toString())
}

/**
 * Serializes a local date time as `yyyy-MM-ddTHH:mm:ss`, without fraction nor offset.
 *
 * Mirrors `ki_utils::serial_date_format`.
 */
object SerialDateSerializer : KSerializer<LocalDateTime> {
    private val FORMAT: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss")

    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("com.kinaro.utils.SerialDate", PrimitiveKind.STRING)

    /**
     * @throws SerializationException if the string does not follow the `yyyy-MM-ddTHH:mm:ss` format
     */
    override fun deserialize(decoder: Decoder): LocalDateTime {
        val value = decoder.decodeString()
        return try {
            LocalDateTime.parse(value, FORMAT)
        } catch (err: DateTimeParseException) {
            throw SerializationException("Invalid date time: $value", err)
        }
    }

    override fun serialize(encoder: Encoder, value: LocalDateTime) = encoder.encodeString(value.format(FORMAT))
}
