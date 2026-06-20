# LMU DualSense Adaptive Trigger Bridge

Le Mans Ultimate의 실시간 telemetry를 Steam DSX UDP 명령으로 변환해 DualSense 적응형 트리거에 전달하는 Windows용 Rust CLI입니다.

v0.2는 실제 `LMU_Data` shared memory 연결, 플레이어 차량 telemetry 추출, 자동 재연결, brake/ABS 및 throttle/TC/rev-limit 효과를 제공합니다.

## 구현된 트리거 피드백

### L2 — 브레이크

- 일반 제동: 브레이크 입력이 깊어질수록 저항 증가
- ABS 개입: 일반 저항을 대체하는 pulse 피드백
- 브레이크 입력이 deadzone 미만이면 Normal

### R2 — 스로틀

- 일반 가속: 스로틀 입력에 비례하는 약한 저항
- TC 개입: 일반 저항을 대체하는 pulse 피드백
- Rev limiter: 최고 RPM 부근에서 vibration 피드백
- 우선순위: Rev limiter → TC → 일반 스로틀 저항 → Normal

LMU 연결이 끊기거나 차량에서 내리면 양쪽 트리거를 Normal로 초기화합니다. 실제 플레이 QA에서 모든 효과와 자동 재연결이 정상 작동하는 것을 확인했습니다. 답력과 pulse 강도 조절 UI는 향후 GUI 버전에서 추가할 예정입니다.

## 실행

Rust가 설치되어 있다면 BAT 파일이 증분 빌드까지 자동으로 처리합니다.

- `run_mock.bat`: LMU와 DSX 없이 mock telemetry와 effect mapping을 콘솔에서 확인
- `run_lmu.bat`: 실제 LMU telemetry를 읽어 Steam DSX로 전송

`run_lmu.bat`을 먼저 실행해도 됩니다. LMU가 없으면 2초 간격으로 기다리며, LMU 종료 후에도 앱을 유지하고 재실행된 게임에 자동 연결합니다.

직접 실행할 수도 있습니다.

```powershell
cargo run -- --telemetry mock --output null
cargo run -- --telemetry lmu --output dsx-udp
cargo run -- --config config.toml
```

설정을 변경하려면 `config.example.toml`을 `config.toml`로 복사하십시오. CLI 옵션은 설정 파일보다 우선합니다.

## Steam DSX

DSX에서 UDP 서버를 활성화해야 합니다. 포트 결정 순서는 다음과 같습니다.

1. CLI `--dsx-port`
2. 설정의 `dsx.port`
3. `C:\Temp\DualSenseX\DualSenseX_PortNumber.txt`
4. fallback `6969`

좌우 트리거는 공식 DSX UDP v2 JSON 형식으로 별도 전송합니다. Resistance는 mode 13, ABS/TC/rev-limit 진동은 강도와 frequency를 전달할 수 있는 Machine mode 18을 사용합니다.

## LMU telemetry

v0.2는 사용자의 실제 조작량을 나타내는 unfiltered throttle/brake/steering/clutch를 사용합니다. ABS와 TC는 LMU의 active 플래그를 별도로 읽습니다.

LMU 공식 lock을 획득한 동안 전체 shared-memory snapshot을 복사한 뒤 안전한 byte parser로 필요한 값만 추출합니다. 게임 헤더와 bindgen은 빌드에 필요하지 않습니다.

## 검증

```powershell
cargo fmt -- --check
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
```

실제 플레이 확인 절차는 [QA Guide](docs/wiki/QA_GUIDE.md)를 참고하십시오.

## 로그 형식

상태 변화는 `[APP]`, `[LMU]`, `[DSX]` 태그로 표시됩니다. 주행 중에는 1초마다 한 줄로 핵심 telemetry와 트리거 효과를 출력합니다.

```text
INFO [LIVE] G  4 | 184 km/h | RPM  7420/8000  | THR  78% | BRK   0% | ABS OFF | TC ON  | L2 NORMAL | R2 PULSE(6)
```

`RESIST`, `PULSE`, `VIBRATE` 뒤의 숫자는 내부 효과 강도 `0..=10`입니다.

## AI-assisted development

이 프로젝트의 코드 대부분은 OpenAI Codex APP을 사용해 작성되었습니다. 설계 검토, 구현, 테스트 작성 및 문서화 과정에 AI가 활용되었으며, 실제 LMU·DSX·DualSense 환경에서의 동작은 사용자가 직접 검증했습니다.
