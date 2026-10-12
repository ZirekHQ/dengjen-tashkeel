package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class TashkeelLoadTest {

    private final Tashkeel engine = new Tashkeel();

    @Test
    void reusingTheSameModelPathReturnsTheLoadedInstance() throws Exception {
        Optional<Path> model = Optional.of(Path.of("/models/a.onnx"));

        assertSame(engine, Tashkeel.reuse(new Tashkeel.Loaded(engine, model), model));
    }

    @Test
    void reusingTheBundledModelReturnsTheLoadedInstance() throws Exception {
        Tashkeel.Loaded loaded = new Tashkeel.Loaded(engine, Optional.empty());

        assertSame(engine, Tashkeel.reuse(loaded, Optional.empty()));
    }

    @Test
    void aDifferentModelPathIsRejectedAsAlreadyInitializedNamingBothModels() {
        Tashkeel.Loaded loaded = new Tashkeel.Loaded(engine, Optional.of(Path.of("/models/a.onnx")));

        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> Tashkeel.reuse(loaded, Optional.of(Path.of("/models/b.onnx"))));

        TashkeelException.AlreadyInitialized reason =
                assertInstanceOf(TashkeelException.AlreadyInitialized.class, exception.reason());
        assertTrue(reason.message().contains("/models/a.onnx"));
        assertTrue(reason.message().contains("/models/b.onnx"));
    }

    @Test
    void switchingBetweenTheBundledAndACustomModelIsRejected() {
        Tashkeel.Loaded loaded = new Tashkeel.Loaded(engine, Optional.empty());

        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> Tashkeel.reuse(loaded, Optional.of(Path.of("/models/a.onnx"))));

        assertInstanceOf(TashkeelException.AlreadyInitialized.class, exception.reason());
    }

    @Test
    void externErrorLayoutMatchesTheNativeStruct() {
        assertEquals(16, TashkeelLib.EXTERN_ERROR.byteSize());
        assertEquals(8, TashkeelLib.EXTERN_ERROR.byteAlignment());
        assertEquals(0, TashkeelLib.EXTERN_ERROR_CODE_OFFSET);
        assertEquals(8, TashkeelLib.EXTERN_ERROR_MESSAGE_OFFSET);
    }
}
