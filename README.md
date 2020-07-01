# MangaMeeya

Project to host the MangaMeeya binaries and codes. Purpose is to reverse engineer the MangaMeeya project for education purposes.

# Design considerations

- Memory-safe as Original MangaMeeya used to crash a lot if loads a big .zip file
- Cross-platform
- Able to leverage on the latest hardware acceleration adn SIMD supprot (x86/ARM/GPU?) (original MM only used SSE/MMX if I'm not mistaken)
- Performance is key and the only thing that matters, boot time/start time must be quick, closing as well, loading the zip, loading the image
- Load all images from zip into RAM only to ensure more speed (Based on the original design of MM)
- Support image codecs to the latest image codecs in the market (WebP, etc)
- Image processing capabilities
- Hotkey is a must
- Windows right click integration?
- Plugins? (Might slow down the overall system)
- Multi-language support
- Browser support (WebASM)
- Multi-threading support
- Android/iOS support?

# Feature in considerations

- URL loading -> no performance gain though?
- Readers are able to submit an edit on the translation back to the community -> But where? And does this flow make sense to ppl?

# Programming language

- Rust (memory-safe language, high performance and supports WebASM for Web, lack Android/iPhone support)

# Debugger used

* x64dbg


