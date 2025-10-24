# 아이콘 & 배포 가이드

## 데스크톱 아이콘
- 저장 위치: `apps/desktop/src-tauri/icons/`
- 포함 형식: `icon.svg`
- SVG 벡터만 사용하며 빌드 시 필요한 모든 플랫폼 자산은 이 파일에서 파생됩니다.

## Android 적응형 아이콘
- 전경 벡터: `android/app/src/main/res/drawable/ic_launcher_foreground.xml`
- 배경 색상: `android/app/src/main/res/values/ic_launcher_background.xml`
- 적응형 매핑: `android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml`
- 전경을 교체할 때는 벡터의 `pathData`만 수정하고 뷰박스 크기(108×108dp)를 유지합니다.
- 배경 색상은 `ic_launcher_background.xml`의 색상 값을 갱신하면 즉시 반영됩니다.

## 빌드 점검표
- [ ] `tauri.conf.json`의 경로가 위 아이콘 파일과 일치하는지 확인합니다.
- [ ] Android 리소스 폴더 구조가 `app/src/main/res/` 하위에 유지되는지 확인합니다.
- [ ] 누락된 아이콘으로 빌드가 실패할 경우 파일 이름, 확장자, 경로를 다시 점검합니다.
