# References

- [Development specification](../LMU-DualSense-Adaptive-Trigger-Bridge.md)
- [pyLMUSharedMemory](https://github.com/TinyPedal/pyLMUSharedMemory)
- [DualSenseX official UDP v2 example](https://github.com/Paliverse/DualSenseX/tree/main/UDP%20Example%20%28C%23%29%20for%20v2.0)
- [Rust standard library UDP socket](https://doc.rust-lang.org/std/net/struct.UdpSocket.html)

DSX v2의 `VibrateTriggerPulse`는 내부 pulse 파라미터를 직접 받지 않으며, `VibrateTrigger`는 intensity만 받는다. 내부 모델의 추가 파라미터는 향후 backend를 위해 유지한다.

