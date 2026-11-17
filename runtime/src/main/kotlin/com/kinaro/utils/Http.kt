package com.kinaro.utils

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder

/** MIME type of a `multipart/form-data` body. */
const val CONTENT_TYPE_FORM_DATA = "multipart/form-data"

/** MIME type of a URL-encoded form body. */
const val CONTENT_TYPE_FORM_URL_ENCODED = "application/x-www-form-urlencoded"

/** MIME type of [ContentType.Json]. */
const val CONTENT_TYPE_JSON = "application/json"

/** MIME type of [ContentType.Text]. */
const val CONTENT_TYPE_PLAIN_TEXT = "text/plain"

/** MIME type of [ContentType.Xml]. */
const val CONTENT_TYPE_XML = "application/xml"

/** MIME type of [ContentType.Csv]. */
const val CONTENT_TYPE_CSV = "text/csv"

/** MIME type of [ContentType.Yaml]. */
const val CONTENT_TYPE_YAML = "application/yaml"

/** Name of the HTTP header giving the media type of the request body. */
const val HTTP_HEADER_CONTENT_TYPE = "Content-Type"

/**
 * Method of a REST request, serialized as its uppercase name (e.g. `"GET"`).
 *
 * Mirrors `ki_utils::http::HttpMethod`.
 */
@Serializable
enum class HttpMethod(val value: String) {
    @SerialName("GET")
    Get("GET"),

    @SerialName("POST")
    Post("POST"),

    @SerialName("PUT")
    Put("PUT"),

    @SerialName("DELETE")
    Delete("DELETE"),

    @SerialName("PATCH")
    Patch("PATCH"),

    @SerialName("HEAD")
    Head("HEAD"),

    @SerialName("OPTIONS")
    Options("OPTIONS"),
}

/**
 * Where a Rest param is placed in the outgoing request, serialized in camelCase (e.g. `"query"`).
 *
 * Mirrors `ki_utils::http::RestParamType`.
 */
@Serializable
enum class RestParamType {
    /** Substituted in the endpoint's path. */
    @SerialName("path")
    Path,

    /** Appended to the URL's query string. */
    @SerialName("query")
    Query,

    /** Sent as an HTTP header. */
    @SerialName("header")
    Header,

    /** Sent in the `Cookie` header. */
    @SerialName("cookie")
    Cookie,
}

/**
 * Content type of raw body, serialized as its MIME string (e.g. `"application/json"`).
 *
 * Deserializing an unknown MIME string gives [ContentType.Text] (see [ContentType.fromMime]).
 *
 * Mirrors `ki_utils::http::ContentType`.
 */
@Serializable(with = ContentTypeSerializer::class)
enum class ContentType(val mime: String) {
    /** `application/json` */
    Json(CONTENT_TYPE_JSON),

    /** `application/xml` */
    Xml(CONTENT_TYPE_XML),

    /** `text/plain`, also the fallback for unknown MIME types. */
    Text(CONTENT_TYPE_PLAIN_TEXT),

    /** `text/csv` */
    Csv(CONTENT_TYPE_CSV),

    /** `application/yaml` */
    Yaml(CONTENT_TYPE_YAML);

    companion object {
        /**
         * Matches known MIME types case-insensitively, falling back to [Text] for any other string
         * (including form MIME types, which are not raw body types).
         */
        fun fromMime(mime: String): ContentType = entries.firstOrNull { it.mime.equals(mime, ignoreCase = true) } ?: Text
    }
}

/** Serializes a [ContentType] as its MIME string. */
object ContentTypeSerializer : KSerializer<ContentType> {
    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("com.kinaro.utils.ContentType", PrimitiveKind.STRING)

    override fun deserialize(decoder: Decoder): ContentType = ContentType.fromMime(decoder.decodeString())

    override fun serialize(encoder: Encoder, value: ContentType) = encoder.encodeString(value.mime)
}
