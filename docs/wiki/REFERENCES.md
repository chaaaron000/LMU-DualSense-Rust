# References

- [Development specification](../LMU-DualSense-Adaptive-Trigger-Bridge.md)
- [pyLMUSharedMemory](https://github.com/TinyPedal/pyLMUSharedMemory), layout 기준 commit `3968c15fc5a127065da2fa0655c9bb0a48ec1b4e`
- [pyLMUSharedMemory lock implementation](https://github.com/TinyPedal/pyLMUSharedMemory/blob/master/lmu_mmap.py)
- [DualSenseX official UDP v2 example](https://github.com/Paliverse/DualSenseX/tree/main/UDP%20Example%20%28C%23%29%20for%20v2.0)
- 로컬 LMU `Support\SharedMemoryInterface` 공식 헤더, 확인일 2026-06-21

## 고정된 layout

- 전체 snapshot: 324,820 bytes
- telemetry 시작 offset: 128,464
- `TelemInfoV01`: 1,888 bytes
- 최대 차량: 104
- packing: 4 bytes

공식 LMU 헤더는 재배포하지 않는다. 빌드 시 헤더나 libclang이 필요하지 않도록 검증된 offset과 안전한 byte parser를 사용한다.

DSX Machine mode 18은 공식 예제의 `start, end, strengthA, strengthB, frequency, period` 순서를 사용하며 end는 9, period는 0으로 고정한다.

