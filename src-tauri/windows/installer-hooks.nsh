!include "LogicLib.nsh"

Var MikasaIconPath
Var MikasaDesktopLnk
Var MikasaStartMenuRootLnk
Var MikasaStartMenuFolderLnk

!macro MikasaSetLinkIcon LNK ICON
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P<> 0
    ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
    ${If} $1 P<> 0
      ${IPersistFile::Load} $1 '("${LNK}", ${STGM_READWRITE})'
      ${IShellLink::SetIconLocation} $0 '(w "${ICON}", i 0)'
      ${IPersistFile::Save} $1 '("${LNK}", 1)'
      ${IUnknown::Release} $1 ""
    ${EndIf}
    ${IUnknown::Release} $0 ""
  ${EndIf}
!macroend

Function MikasaRefreshShortcutIcons
  ${If} $MikasaIconPath == ""
    Return
  ${EndIf}
  ${IfNot} ${FileExists} "$MikasaIconPath"
    Return
  ${EndIf}

  ${If} $MikasaDesktopLnk != ""
  ${AndIf} ${FileExists} "$MikasaDesktopLnk"
    !insertmacro MikasaSetLinkIcon "$MikasaDesktopLnk" "$MikasaIconPath"
  ${EndIf}

  ${If} $MikasaStartMenuRootLnk != ""
  ${AndIf} ${FileExists} "$MikasaStartMenuRootLnk"
    !insertmacro MikasaSetLinkIcon "$MikasaStartMenuRootLnk" "$MikasaIconPath"
  ${EndIf}

  ${If} $MikasaStartMenuFolderLnk != ""
  ${AndIf} ${FileExists} "$MikasaStartMenuFolderLnk"
    !insertmacro MikasaSetLinkIcon "$MikasaStartMenuFolderLnk" "$MikasaIconPath"
  ${EndIf}
FunctionEnd

Function .onGUIEnd
  Call MikasaRefreshShortcutIcons
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
  StrCpy $MikasaIconPath "$INSTDIR\app-icon.ico"
  StrCpy $MikasaDesktopLnk "$DESKTOP\${PRODUCTNAME}.lnk"
  StrCpy $MikasaStartMenuRootLnk "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  StrCpy $MikasaStartMenuFolderLnk ""

  ${If} $AppStartMenuFolder != ""
  ${AndIf} $AppStartMenuFolder != ">"
    StrCpy $MikasaStartMenuFolderLnk "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  ${EndIf}

  Call MikasaRefreshShortcutIcons
!macroend
