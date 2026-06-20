# 개발 명세서: LMU DualSense Adaptive Trigger Bridge

## 1. 프로젝트 개요

Le Mans Ultimate의 실시간 텔레메트리를 읽어서 DualSense 적응형 트리거 효과로 변환하는 Windows용 Rust 프로그램을 구현한다.

초기 버전은 DualSense를 직접 HID로 제어하지 않는다. 대신 DSX/DualSenseX의 UDP 입력 기능을 통해 adaptive trigger 명령을 전송한다.

최종 목표는 다음 파이프라인을 구현하는 것이다.

```text
Le Mans Ultimate
→ LMU Shared Memory
→ Rust Telemetry Reader
→ Trigger Effect Mapper
→ DSX UDP Output
→ DualSense Adaptive Trigger
```

추가 목표:

1. 일반 사용자가 쉽게 실행할 수 있도록 `.bat` 실행 파일을 제공한다.
2. 프로젝트 구조, 개발 과정, 참고 문서, 구현 결정 사항은 별도의 LLM Wiki 형태로 관리한다.
3. 실제 플레이 기반 QA 단계에서는 사용자가 어떤 상황에서 무엇을 확인해야 하는지 명확한 체크리스트를 제공한다.

---

## 2. 개발 목표

### v0.1 목표

1. Rust CLI 프로그램 생성
2. 설정 파일 로딩
3. Mock telemetry reader 구현
4. LMU shared memory reader 인터페이스 설계
5. throttle / brake / rpm / gear / ABSActive / TCActive 값을 `TelemetryFrame`으로 표준화
6. telemetry 값을 콘솔에 주기적으로 출력
7. DSX UDP output 인터페이스와 기본 구현 추가
8. DSX로 좌우 트리거 기본 명령을 보낼 수 있는 구조 구현
9. 실행용 BAT 파일 제공
10. LLM Wiki 초안 작성

### v0.2 목표

1. 실제 LMU shared memory 연결
2. LMU `LMU_Data` memory map 열기
3. player vehicle telemetry 읽기
4. brake 입력값에 따라 L2 저항 증가
5. ABS 활성 시 L2 pulse/vibration 효과
6. throttle 입력값에 따라 R2 약한 저항
7. TC 활성 시 R2 pulse/vibration 효과
8. RPM이 rev limit 근처일 때 R2 vibration 효과
9. 설정 파일로 효과 강도 조절
10. 실제 플레이 QA 체크리스트 작성

### v1.0 이후 목표

1. tray app 또는 간단 GUI
2. 프리셋 저장/로드
3. 직접 DualSense HID backend
4. 게임별 telemetry backend 확장
5. LMU 외 rFactor 2, Assetto Corsa Competizione 등 확장 가능 구조

---

## 3. 명확한 제외 범위

초기 버전에서는 아래 기능을 구현하지 않는다.

1. DualSense 직접 HID 제어
2. Bluetooth DualSense 직접 제어
3. Steam Input / DS4Windows / DSX 충돌 자동 해결
4. GUI
5. installer
6. telemetry logging
7. 리플레이 분석
8. REST API 기반 제어
9. LMU 외 다른 게임 지원

초기 목표는 오직 다음이다.

```text
LMU 실시간 shared memory 읽기
→ 필요한 값 추출
→ DSX UDP로 adaptive trigger 명령 전송
```

---

## 4. 타겟 환경

```text
OS: Windows 10/11
Target: x86_64-pc-windows-msvc
Language: Rust
Edition: Rust 2021
Runtime: Native CLI app
Game: Le Mans Ultimate
Controller path: DSX / DualSenseX 경유
```

---

## 5. 권장 dependency

`Cargo.toml`에 다음 dependency를 우선 사용한다.

```toml
[package]
name = "lmu-dualsense-bridge"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
thiserror = "1"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4", features = ["derive"] }

shared_memory = "0.12"

[build-dependencies]
bindgen = "0.72"
```

주의:

* LMU Shared Memory 구조체 정의는 공식 헤더보다 실제 동작 검증이 되어 있는 pyLMUSharedMemory 프로젝트를 우선 참고한다.
* 참고 레포지토리: https://github.com/TinyPedal/pyLMUSharedMemory
* LMU 구조 변경 여부를 확인할 때 pyLMUSharedMemory의 최신 구현과 필드 매핑을 검증 기준으로 사용한다.
* 필요 시 bindgen 기반 접근 대신 pyLMUSharedMemory에서 정의한 구조를 참고하여 Rust 구조체를 구현할 수 있다.
* header 경로를 찾지 못하는 환경에서도 빌드 가능하도록 `mock` feature 또는 fallback을 둔다.
* v0.1 mock 구현에서는 간략한 placeholder 구조체를 허용한다.

---

## 6. 프로젝트 구조

다음 구조로 작성한다.

```text
lmu-dualsense-bridge/
  Cargo.toml
  build.rs
  config.example.toml
  README.md

  run_mock.bat
  run_lmu.bat

  docs/
    wiki/
      INDEX.md
      ARCHITECTURE.md
      DEVELOPMENT_LOG.md
      REFERENCES.md
      QA_GUIDE.md
      TROUBLESHOOTING.md

  src/
    main.rs
    app.rs
    config.rs

    telemetry/
      mod.rs
      frame.rs
      reader.rs
      mock_reader.rs
      lmu_shared_memory_reader.rs

    effects/
      mod.rs
      mapper.rs
      smoothing.rs
      trigger.rs

    output/
      mod.rs
      dsx_udp.rs
      null_output.rs

    util/
      mod.rs
      rate_limiter.rs
```

---

## 7. 핵심 데이터 모델

### 7.1 TelemetryFrame

게임별 raw telemetry를 앱 내부 표준 형식으로 변환한다.

```rust
#[derive(Debug, Clone, Copy)]
pub struct TelemetryFrame {
    pub connected: bool,
    pub player_has_vehicle: bool,

    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub steering: f32,

    pub rpm: f32,
    pub max_rpm: f32,
    pub gear: i32,

    pub abs_active: bool,
    pub tc_active: bool,

    pub abs_level: i32,
    pub tc_level: i32,
    pub tc_slip_level: i32,
    pub tc_cut_level: i32,

    pub speed_mps: f32,
    pub pit_limiter_active: bool,
}
```

### 7.2 TelemetryReader trait

```rust
pub trait TelemetryReader {
    fn poll(&mut self) -> anyhow::Result<TelemetryFrame>;
}
```

구현체:

```rust
MockTelemetryReader
LmuSharedMemoryReader
```

### 7.3 TriggerEffect

DSX 또는 HID backend와 무관한 내부 트리거 표현이다.

```rust
#[derive(Debug, Clone)]
pub struct TriggerOutputFrame {
    pub left: TriggerEffect,
    pub right: TriggerEffect,
}

#[derive(Debug, Clone)]
pub enum TriggerEffect {
    Normal,

    Resistance {
        start: u8,
        force: u8,
    },

    Pulse {
        start: u8,
        force: u8,
        frequency: u8,
    },

    Vibrate {
        start: u8,
        force: u8,
        frequency: u8,
    },
}
```

값 범위는 우선 `0..=10` 또는 `0..=255` 중 하나로 통일한다.

v0.1에서는 내부 값 범위를 `0..=10`으로 두고, DSX encoder에서 DSX 명령 범위로 변환한다.

---

## 8. 설정 파일

`config.example.toml`을 작성한다.

```toml
[app]
tick_hz = 60
telemetry_source = "mock"
output = "null"
log_level = "info"

[lmu]
shared_memory_name = "LMU_Data"
header_path = ""

[dsx]
host = "127.0.0.1"
port = 6969
protocol = "legacy_text"

[effects.brake]
enabled = true
deadzone = 0.03
min_force = 1
max_force = 8
start_position = 2
abs_pulse_force = 9
abs_pulse_frequency = 8

[effects.throttle]
enabled = true
deadzone = 0.03
min_force = 0
max_force = 4
start_position = 2
tc_pulse_force = 6
tc_pulse_frequency = 7

[effects.rpm]
enabled = true
rev_limit_ratio = 0.97
vibration_force = 7
vibration_frequency = 10

[smoothing]
enabled = true
attack = 0.45
release = 0.25
```

---

## 9. Effect mapping 규칙

`EffectMapper`는 `TelemetryFrame`을 받아 `TriggerOutputFrame`을 반환한다.

```rust
pub struct EffectMapper {
    config: EffectConfig,
    previous: Option<TelemetryFrame>,
}
```

### 9.1 L2 브레이크 규칙

입력:

```text
brake = frame.brake
abs_active = frame.abs_active
```

규칙:

1. `brake < deadzone`이면 `TriggerEffect::Normal`
2. `brake >= deadzone`이면 `Resistance`
3. brake 값이 커질수록 force 증가
4. `abs_active == true`이면 `Pulse` 또는 `Vibrate`로 override
5. ABS 효과는 설정 파일로 조절 가능해야 한다

### 9.2 R2 스로틀 규칙

입력:

```text
throttle = frame.throttle
tc_active = frame.tc_active
rpm_ratio = frame.rpm / frame.max_rpm
```

우선순위:

```text
rev limiter vibration
> TC active pulse
> throttle resistance
> normal
```

규칙:

1. `rpm_ratio >= rev_limit_ratio`이면 R2 vibration
2. `tc_active == true`이면 R2 pulse
3. `throttle >= deadzone`이면 약한 resistance
4. 그 외 normal

---

## 10. DSX UDP Output

### 10.1 Output trait

```rust
pub trait TriggerOutput {
    fn send(&mut self, frame: &TriggerOutputFrame) -> anyhow::Result<()>;
}
```

구현체:

```rust
NullOutput
DsxUdpOutput
```

### 10.2 NullOutput

개발 및 테스트용 output이다.

`TriggerOutputFrame`을 로그로 출력한다.

### 10.3 DsxUdpOutput

```rust
pub struct DsxUdpOutput {
    socket: UdpSocket,
    target: SocketAddr,
    encoder: DsxPacketEncoder,
}
```

주의:

* DSX 프로토콜 구현은 별도 encoder에 격리한다.
* DSX가 실행 중이 아니어도 앱은 종료되지 않아야 한다.
* UDP 전송 실패는 warning 로그로 처리한다.

---

## 11. LMU Shared Memory Reader

### 11.1 기본 방침

LMU shared memory는 `LMU_Data` named shared memory를 읽는다.

구현 순서:

1. `shared_memory` crate로 `LMU_Data` 열기 시도
2. 실패 시 명확한 에러 메시지 출력
3. memory map byte slice 확보
4. pyLMUSharedMemory 레포지토리의 구조 정의를 기준으로 데이터 해석
5. `playerHasVehicle` 확인
6. `playerVehicleIdx`로 플레이어 차량 telemetry 선택
7. 필요한 값만 `TelemetryFrame`으로 변환

### 11.2 구조체 정의 기준

LMU Shared Memory 구조는 아래 레포지토리를 공식 참고 자료로 사용한다.

```text
https://github.com/TinyPedal/pyLMUSharedMemory
```

방침:

1. pyLMUSharedMemory의 최신 구조체 정의를 우선 참고한다.
2. LMU 업데이트 시 pyLMUSharedMemory 변경 내역을 확인한다.
3. Rust 구조체 필드명은 가능한 한 pyLMUSharedMemory와 대응되도록 유지한다.
4. 필요한 최소 필드만 추출하여 `TelemetryFrame`으로 변환한다.
5. bindgen 사용은 선택 사항이며 필수 요구사항이 아니다.
6. 실제 구현 시 pyLMUSharedMemory의 Python ctypes 구조를 Rust 구조체로 대응시킨다.

### 11.3 안전성

규칙:

1. `unsafe`는 LMU reader 내부에만 허용
2. public API는 safe wrapper만 노출
3. memory size 검증 필수
4. player index 범위 검증 필수
5. 연결 실패 시 graceful fallback

---

## 12. App loop

```rust
pub struct App {
    reader: Box<dyn TelemetryReader>,
    mapper: EffectMapper,
    output: Box<dyn TriggerOutput>,
    tick_hz: u32,
}
```

동작:

```text
loop:
  frame = reader.poll()
  trigger_frame = mapper.map(frame)
  output.send(trigger_frame)
  sleep_until_next_tick
```

조건:

1. 기본 tick rate 60Hz
2. busy loop 금지
3. panic 최소화
4. LMU 미실행 상태에서도 대기 가능

---

## 13. CLI

`clap` 사용.

예시:

```text
lmu-dualsense-bridge --config config.toml
lmu-dualsense-bridge --telemetry mock --output null
lmu-dualsense-bridge --telemetry lmu --output dsx_udp
```

CLI override는 config보다 우선한다.

---

## 14. BAT 실행 파일 요구사항

일반 사용자가 Rust CLI 옵션을 직접 입력하지 않아도 되도록 BAT 파일을 제공한다.

### run_mock.bat

```bat
@echo off
lmu-dualsense-bridge.exe --telemetry mock --output null
pause
```

### run_lmu.bat

```bat
@echo off
lmu-dualsense-bridge.exe --telemetry lmu --output dsx_udp
pause
```

요구사항:

1. README에서 BAT 실행 방법 설명
2. 개발 빌드용 BAT와 배포용 BAT 분리 가능
3. 오류 발생 시 콘솔이 즉시 닫히지 않도록 `pause` 사용

---

## 15. Logging

`tracing` 사용.

필수 로그:

1. config 로드 성공/실패
2. telemetry source
3. output 종류
4. LMU 연결 성공/실패
5. DSX target address
6. telemetry 요약 출력

---

## 16. 테스트

### 16.1 Unit test

다음 테스트 작성:

1. `map_brake_to_force`
2. `map_throttle_to_force`
3. ABS active → L2 Pulse
4. TC active → R2 Pulse
5. Rev limiter → R2 Vibrate
6. Deadzone → Normal
7. Config parsing

### 16.2 Mock integration test

시나리오:

1. idle
2. brake ramp
3. ABS pulse
4. throttle ramp
5. TC pulse
6. rev limiter vibration
7. gear change

---

## 17. 실제 플레이 QA 가이드

실제 플레이 QA 단계에서는 반드시 사용자가 직접 확인 가능한 체크리스트를 제공한다.

### QA-01 기본 연결 확인

상황:

* LMU 실행
* DSX 실행
* 차량 탑승 상태

확인:

* LMU 연결 로그 출력
* DSX UDP 전송 로그 출력
* 앱이 종료되지 않음

기대 결과:

* 연결 성공 메시지 확인

### QA-02 브레이크 저항 확인

상황:

* 직선 구간
* ABS 개입 없는 일반 제동

확인:

* 브레이크를 깊게 밟을수록 L2 저항 증가

기대 결과:

* 브레이크 입력량에 비례한 저항

### QA-03 ABS 개입 확인

상황:

* 강한 제동
* ABS 차량

확인:

* ABS 개입 시 L2 펄스 또는 진동 발생

기대 결과:

* 일반 저항보다 명확한 진동 느낌

### QA-04 스로틀 저항 확인

상황:

* 코너 탈출
* 정상 가속

확인:

* R2에 약한 저항 발생

기대 결과:

* 스로틀 입력량에 비례한 저항

### QA-05 TC 개입 확인

상황:

* 저속 코너 탈출
* TC 개입 유도

확인:

* R2 펄스 발생

기대 결과:

* TC 개입 시 명확한 피드백

### QA-06 Rev Limiter 확인

상황:

* 최고 RPM 근처 유지

확인:

* R2 진동 발생

기대 결과:

* 변속 타이밍을 촉각으로 인지 가능

---

## 18. LLM Wiki 요구사항

프로젝트 루트에 `docs/wiki` 디렉토리를 유지한다.

### INDEX.md

전체 문서 목차

### ARCHITECTURE.md

시스템 구조

```text
LMU
→ Shared Memory
→ Telemetry Reader
→ Effect Mapper
→ DSX Output
→ DualSense
```

### DEVELOPMENT_LOG.md

개발 진행 상황 기록

### REFERENCES.md

참고 문서 링크

예시:

* pyLMUSharedMemory: https://github.com/TinyPedal/pyLMUSharedMemory
* DSX UDP 문서
* Rust crate 문서

### QA_GUIDE.md

실제 플레이 QA 절차

### TROUBLESHOOTING.md

문제 해결 가이드

요구사항:

1. 구현 변경 시 Wiki 업데이트
2. 중요한 설계 결정 기록
3. 참고 링크 기록
4. QA 결과 기록

---

## 19. Acceptance Criteria

### v0.1 완료 기준

1. `cargo build` 성공
2. `cargo fmt` 적용
3. `cargo clippy` 주요 warning 없음
4. `cargo test` 성공
5. mock telemetry 출력
6. null output 출력
7. DSX UDP 전송 시도
8. DSX 미실행 상태에서도 정상 동작
9. BAT 파일 제공
10. LLM Wiki 초안 작성

### v0.2 완료 기준

1. LMU shared memory 연결
2. player telemetry 추출
3. 브레이크 저항 생성
4. ABS 효과 생성
5. 스로틀 저항 생성
6. TC 효과 생성
7. Rev limiter 효과 생성
8. DSX 실제 동작 확인
   9
