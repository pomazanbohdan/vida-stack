import ../../config/loader

proc cmdStatus*(args: seq[string]): int =
  discard args
  echo "VIDA v0 Runtime v0.2.0"
  echo "VIDA_ROOT: " & vidaRoot()
  echo "VIDA_WORKSPACE: " & vidaWorkspaceDir()
  echo "Config: " & configPath()
  0
