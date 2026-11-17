@file:OptIn(ExperimentalSerializationApi::class)

package com.kinaro.project

import com.kinaro.utils.ContentType
import com.kinaro.utils.HttpMethod
import com.kinaro.utils.RestParamType
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonClassDiscriminator

/** A name/value pair sent as part of the request. */
@Serializable
data class RestParam(
    @SerialName("type")
    val paramType: RestParamType,
    val name: String,
    val value: String,
    val disabled: Boolean = false,
)

/** A field of a `application/x-www-form-urlencoded` body. */
@Serializable
data class FormField(
    val name: String,
    val value: String,
    val disabled: Boolean = false,
)

/** A part of a `multipart/form-data` body, tagged by `"kind"`. */
@Serializable
@JsonClassDiscriminator("kind")
sealed interface MultipartPart {
    @Serializable
    @SerialName("Text")
    data class Text(
        val name: String,
        val value: String,
        val disabled: Boolean = false,
    ) : MultipartPart

    @Serializable
    @SerialName("File")
    data class File(
        val name: String,
        val path: String,
        /** Content type of the part, guessed from the file when absent. */
        val contentType: String? = null,
        val disabled: Boolean = false,
    ) : MultipartPart
}

/** The request body, tagged by `"type"`. Its variant determines the `Content-Type` sent with the request. */
@Serializable
@JsonClassDiscriminator("type")
sealed interface RestBody {
    @Serializable
    @SerialName("none")
    data object None : RestBody

    /** Free text (JSON, XML, plain text...) sent as-is. */
    @Serializable
    @SerialName("raw")
    data class Raw(
        val contentType: ContentType,
        /**
         * One entry per line, so version control diffs only the lines that
         * actually changed instead of one escaped multiline string.
         */
        val lines: List<String>,
    ) : RestBody

    @Serializable
    @SerialName("formUrlEncoded")
    data class FormUrlEncoded(val fields: List<FormField>) : RestBody

    @Serializable
    @SerialName("multipart")
    data class Multipart(val parts: List<MultipartPart>) : RestBody
}

/** On-disk form of a REST step. */
@Serializable
data class RestData(
    val method: HttpMethod,
    val endpoint: String,
    val params: List<RestParam> = emptyList(),
    val body: RestBody = RestBody.None,
)
