@file:OptIn(ExperimentalSerializationApi::class)

package com.kinaro.project

import com.kinaro.utils.SerialDateSerializer
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.decodeFromStream
import java.io.IOException
import java.nio.file.Path
import java.time.LocalDateTime
import kotlin.io.path.exists
import kotlin.io.path.inputStream

/**
 * JSON format of a project file.
 */
val ProjectJson: Json = Json {
    ignoreUnknownKeys = true
}

/**
 * On-disk form of a project (`.kpr` JSON file).
 *
 * Mirrors `ki_project::ProjectFile`.
 */
@Serializable
data class Project(
    val name: String,
    val version: UShort,
    @Serializable(with = SerialDateSerializer::class)
    val created: LocalDateTime,
    @Serializable(with = SerialDateSerializer::class)
    val modified: LocalDateTime,
    val variables: ProjectVariables,
    val tests: TestsContainer,
) {
    companion object {
        /**
         * Reads the project stored at [path].
         *
         * @throws ProjectError.BadLocation if [path] is not an existing file
         * @throws ProjectError.Io if the file cannot be read
         * @throws ProjectError.Read if the file content is not a valid project
         */
        fun load(path: Path): Project {
            if(!path.exists()) {
                throw ProjectError.BadLocation(path)
            }
            return try {
                path.inputStream().use { ProjectJson.decodeFromStream<Project>(it) }
            } catch (err: IOException) {
                throw ProjectError.Io(err)
            } catch (err: IllegalArgumentException) {
                // SerializationException is an IllegalArgumentException, as are invalid values (e.g. out of range numbers)
                throw ProjectError.Read(err.message.orEmpty(), err)
            }
        }
    }
}
