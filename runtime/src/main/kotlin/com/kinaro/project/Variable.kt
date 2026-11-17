@file:UseSerializers(UuidSerializer::class)

package com.kinaro.project

import com.kinaro.utils.UuidSerializer
import kotlinx.serialization.Serializable
import kotlinx.serialization.UseSerializers
import java.util.*

/** The variables of a project, and the profiles able to override their values. */
@Serializable
data class ProjectVariables(
    val variables: List<Variable> = emptyList(),
    val profiles: List<Profile> = emptyList(),
)

/**
 * A project variable.
 *
 * [overrides] maps a [Profile.id] to the value the variable takes in this profile.
 */
@Serializable
data class Variable(
    val id: UUID,
    val name: String,
    val description: String,
    val kind: VariableKind,
    val value: String,
    val overrides: Map<UUID, String>,
)

@Serializable
enum class VariableKind {
    Text,
    PasswordClear,
    PasswordEncrypt,
}

/**
 * A profile, overriding the values of some variables.
 *
 * Two profiles are equal when their ids are, and are ordered by name.
 */
@Serializable
data class Profile(
    val id: UUID,
    val name: String,
    val description: String,
)