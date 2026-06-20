# Architecture

```text
TelemetryReader
  MockTelemetryReader ─┐
  LmuSharedMemoryReader┴→ TelemetryFrame
                         → EffectMapper
                         → TriggerOutputFrame
                         → NullOutput / DsxUdpOutput
```

게임 및 출력 backend는 trait 뒤에 격리한다. 내부 트리거 강도는 `0..=10`이고 DSX encoder만 프로토콜 범위로 변환한다.

v0.1의 LMU reader는 safe scaffold다. v0.2에서 named mapping 열기, 크기 검증, player index 검증 및 raw layout 해석을 reader 내부에 추가한다.

