; Mark-Hulk NSIS installer hooks.
;
; Tauri associates .md/.markdown/.mdx with the "Markdown Document" file class
; and points its DefaultIcon at the application exe. We repoint it to a dedicated
; document icon (filetypes.ico, shipped as a resource) so markdown files show
; their own icon in Explorer — like .psd/.ai do — distinct from the app logo.
!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHELL_CONTEXT "Software\Classes\Markdown Document\DefaultIcon" "" "$INSTDIR\filetypes.ico"
  ; Ask the shell to drop its cached file-type icons so the change shows at once.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
!macroend
