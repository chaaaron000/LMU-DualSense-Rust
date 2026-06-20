# Troubleshooting

## BAT에서 빌드하지 못함

BAT는 실행 전에 자동으로 `cargo build`를 수행한다. Cargo를 찾지 못하면 [rustup](https://rustup.rs/)으로 Rust를 설치하고 새 콘솔에서 다시 실행한다.

## LMU 연결 로그가 나오지 않음

- Le Mans Ultimate 프로세스 이름이 `Le Mans Ultimate.exe`인지 확인한다.
- 게임 설치 파일의 `Support\SharedMemoryInterface`가 존재하는지 확인한다.
- 브리지는 2초마다 자동 재연결하므로 LMU를 나중에 실행해도 된다.
- 권한이 다른 프로세스 사이에서 mapping 접근이 거부되면 LMU와 브리지를 같은 사용자 권한으로 실행한다.

## LMU layout validation failed

LMU 업데이트로 shared-memory layout이 변경되었을 수 있다. 게임의 최신 공식 헤더와 pyLMUSharedMemory 구조를 비교하고 snapshot 크기와 offset을 갱신해야 한다.

## DSX 반응 없음

- DSX UDP 서버가 활성화되었는지 확인한다.
- `C:\Temp\DualSenseX\DualSenseX_PortNumber.txt` 또는 `--dsx-port` 값을 확인한다.
- 기본 controller index는 0이다.

## Pulse/Vibration 느낌이 이상함

v0.2는 DSX Machine mode 18을 사용한다. 실기에서 호환되지 않으면 Pulse는 preset mode 11, Vibrate는 mode 8로 fallback하는 후속 수정이 필요하다.
