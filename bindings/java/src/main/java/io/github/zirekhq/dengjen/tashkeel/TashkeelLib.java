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
import java.util.Optional;

final class TashkeelLib {

    static final GroupLayout EXTERN_ERROR = MemoryLayout.structLayout(
                    JAVA_INT.withName("code"), MemoryLayout.paddingLayout(4), ADDRESS.withName("message"))
            .withName("ExternError");
    static final long EXTERN_ERROR_CODE_OFFSET = EXTERN_ERROR.byteOffset(PathElement.groupElement("code"));
    static final long EXTERN_ERROR_MESSAGE_OFFSET = EXTERN_ERROR.byteOffset(PathElement.groupElement("message"));

    /** {@code initOrtDylib} is present only when the native library was built with {@code ort-dylib}. */
    record Handles(
            MethodHandle init, Optional<MethodHandle> initOrtDylib, MethodHandle tashkeel, MethodHandle freeString) {
    }

    private static Handles handles;

    static synchronized Handles handles() {
        if (handles == null) {
            handles = bind(NativeLibraryLoader.load());
        }
        return handles;
    }

    private static Handles bind(SymbolLookup lookup) {
        return new Handles(
                handle(lookup, "dengjen_tashkeel_init", FunctionDescriptor.ofVoid(ADDRESS, ADDRESS)),
                optionalHandle(lookup, "dengjen_tashkeel_init_ort_dylib", FunctionDescriptor.ofVoid(ADDRESS, ADDRESS)),
                handle(lookup, "dengjenTashkeelTashkeel",
                        FunctionDescriptor.of(ADDRESS, ADDRESS, ADDRESS, JAVA_BOOLEAN, ADDRESS)),
                handle(lookup, "dengjen_tashkeel_free_string", FunctionDescriptor.ofVoid(ADDRESS)));
    }

    private static Optional<MethodHandle> optionalHandle(
            SymbolLookup lookup, String symbol, FunctionDescriptor descriptor) {
        return lookup.find(symbol).map(address -> Linker.nativeLinker().downcallHandle(address, descriptor));
    }

    private static MethodHandle handle(SymbolLookup lookup, String symbol, FunctionDescriptor descriptor) {
        return Linker.nativeLinker().downcallHandle(
                lookup.find(symbol).orElseThrow(() -> new IllegalStateException("missing symbol: " + symbol)),
                descriptor);
    }

    private TashkeelLib() {
    }
}
