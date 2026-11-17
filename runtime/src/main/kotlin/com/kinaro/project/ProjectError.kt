package com.kinaro.project

import java.io.IOException
import java.nio.file.Path

private const val PROJECT_FILE_ERROR = "ProjectError"

/**
 * Errors raised while creating, loading or saving a [Project].
 *
 * Mirrors `ki_project::ProjectFileError`, messages included.
 */
sealed class ProjectError(message: String, cause: Throwable? = null) : Exception(message, cause) {
    /** General Io error */
    class Io(error: IOException) : ProjectError("$PROJECT_FILE_ERROR::Io (err: ${error.message})", error)

    /** A read deserialization has failed */
    class Read(error: String, cause: Throwable? = null) :
        ProjectError("$PROJECT_FILE_ERROR::Read (err: \"$error\")", cause)

    /** Invalid path, does not exist or cannot be written to */
    class BadLocation(path: Path) : ProjectError("$PROJECT_FILE_ERROR::BadLocation (path: \"$path\")")
}
