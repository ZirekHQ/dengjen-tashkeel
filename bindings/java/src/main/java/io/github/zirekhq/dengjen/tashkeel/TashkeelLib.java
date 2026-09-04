package io.github.zirekhq.dengjen.tashkeel;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_BOOLEAN;
import static java.lang.foreign.ValueLayout.JAVA_INT;

import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.GroupLayout;
import java.lang.foreign.Linker;
import java.lang.foreign.MemoryLayout;
import java.lang.foreign.MemoryLayout.PathElement;
import java.lang.foreign.SymbolLookup;
import java.lang.invoke.MethodHandle;

final class TashkeelLib {

    private static final Linker LINKER = Linker.nativeLinker();
    private static final SymbolLookup LOOKUP = NativeLibraryLoader.load();

    static final GroupLayout EXTERN_ERROR = MemoryLayout.structLayout(
                    JAVA_INT.withName("code"), MemoryLayout.paddingLayout(4), ADDRESS.withName("message"))
            .withName("ExternError");
    static final long EXTERN_ERROR_CODE_OFFSET = EXTERN_ERROR.byteOffset(PathElement.groupElement("code"));
    static final long EXTERN_ERROR_MESSAGE_OFFSET = EXTERN_ERROR.byteOffset(PathElement.groupElement("message"));

    static final MethodHandle INIT =
            handle("dengjen_tashkeel_init", FunctionDescriptor.ofVoid(ADDRESS, ADDRESS));

    static final MethodHandle TASHKEEL = handle(
            "dengjenTashkeelTashkeel", FunctionDescriptor.of(ADDRESS, ADDRESS, ADDRESS, JAVA_BOOLEAN, ADDRESS));

    static final MethodHandle FREE_STRING =
            handle("dengjen_tashkeel_free_string", FunctionDescriptor.ofVoid(ADDRESS));

    private static MethodHandle handle(String symbol, FunctionDescriptor descriptor) {
        return LINKER.downcallHandle(
                LOOKUP.find(symbol).orElseThrow(() -> new IllegalStateException("missing symbol: " + symbol)),
                descriptor);
    }

    private TashkeelLib() {
    }
}
