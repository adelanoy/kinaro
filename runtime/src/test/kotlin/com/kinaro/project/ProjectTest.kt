package com.kinaro.project

import com.kinaro.utils.ContentType
import com.kinaro.utils.HttpMethod
import com.kinaro.utils.RestParamType
import com.kinaro.utils.TimeUnit
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.jsonObject
import java.nio.file.Files
import java.util.UUID
import kotlin.io.path.div
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class ProjectTest {
    private fun json(text: String): JsonElement = Json.parseToJsonElement(text)

    private inline fun <reified T> roundtrip(value: T): T =
        ProjectJson.decodeFromString(ProjectJson.encodeToString(value))

    private fun param(type: RestParamType, name: String, value: String) = RestParam(type, name, value)

    @Test
    fun minimalStepOmitsParamsAndBody() {
        val step = RestData(HttpMethod.Get, "/users")

        val value = ProjectJson.encodeToJsonElement(step)

        assertEquals(json("""{ "method": "GET", "endpoint": "/users" }"""), value)
        assertEquals(step, ProjectJson.decodeFromJsonElement(value))
    }

    @Test
    fun paramsSerializeWithTypeAndSkipDisabledWhenFalse() {
        val step = RestData(
            HttpMethod.Get,
            "/users/{id}",
            params = listOf(param(RestParamType.Path, "id", "42"), param(RestParamType.Header, "X-Trace", "1").copy(disabled = true)),
        )

        val value = ProjectJson.encodeToJsonElement(step)

        assertEquals(
            json(
                """[
                { "type": "path", "name": "id", "value": "42" },
                { "type": "header", "name": "X-Trace", "value": "1", "disabled": true }
            ]"""
            ),
            value.jsonObject["params"],
        )
        assertEquals(step, roundtrip(step))
    }

    @Test
    fun rawBodyIsTaggedAndStoredByLine() {
        val step = RestData(HttpMethod.Post, "/users", body = RestBody.Raw(ContentType.Json, listOf("{", "}")))

        val value = ProjectJson.encodeToJsonElement(step)

        assertEquals(
            json("""{ "type": "raw", "contentType": "application/json", "lines": ["{", "}"] }"""),
            value.jsonObject["body"],
        )
        assertEquals(step, roundtrip(step))
    }

    @Test
    fun formAndMultipartBodiesRoundtrip() {
        for (body in listOf(
            RestBody.FormUrlEncoded(listOf(FormField("a", "1"))),
            RestBody.Multipart(
                listOf(
                    MultipartPart.Text("title", "doc"),
                    MultipartPart.File("file", "data/doc.pdf", "application/pdf", disabled = true),
                )
            ),
        )) {
            val step = RestData(HttpMethod.Put, "/upload", body = body)
            assertEquals(step, roundtrip(step))
        }
    }

    @Test
    fun multipartPartIsTaggedByKind() {
        val part: MultipartPart = MultipartPart.File("f", "a.txt")

        assertEquals(json("""{ "kind": "File", "name": "f", "path": "a.txt" }"""), ProjectJson.encodeToJsonElement(part))
    }

    @Test
    fun unknownContentTypeFallsBackToText() {
        val body = ProjectJson.decodeFromString<RestBody>("""{ "type": "raw", "contentType": "image/png", "lines": [] }""")

        assertEquals(RestBody.Raw(ContentType.Text, emptyList()), body)
    }

    @Test
    fun suiteChildrenAreTaggedByKind() {
        val (suiteId, caseId, stepId) = List(3) { UUID.randomUUID() }
        val step = TestStep(stepId, "Step", data = StepData.Delay(DelayData(10u, TimeUnit.Second)))
        val suite = TestSuite(
            suiteId,
            "Suite",
            children = listOf(TestCase(caseId, "Case", steps = listOf(step)), step),
        )

        val value = ProjectJson.encodeToJsonElement(suite)

        val stepValue = """{ "id": "$stepId", "name": "Step", "data": { "Delay": { "duration": 10, "unit": "Second" } } }"""
        assertEquals(
            json(
                """{
                "id": "$suiteId",
                "name": "Suite",
                "children": [
                    { "type": "Case", "id": "$caseId", "name": "Case", "steps": [$stepValue] },
                    { "type": "Step", "id": "$stepId", "name": "Step", "data": { "Delay": { "duration": 10, "unit": "Second" } } }
                ]
            }"""
            ),
            value,
        )
        assertEquals(suite, ProjectJson.decodeFromJsonElement(value))
    }

    @Test
    fun invalidContentIsAReadError() {
        val path = Files.createTempDirectory("kinaro") / "project.kpr"
        Files.writeString(path, """{ "name": "Project" }""")

        assertFailsWith<ProjectError.Read> { Project.load(path) }
    }
}
