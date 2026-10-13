def project_legacy(actual,expected):
 if isinstance(expected,dict):
  assert isinstance(actual,dict)
  assert expected.keys()<=actual.keys()
  return {k:project_legacy(actual[k],v) for k,v in expected.items()}
 if isinstance(expected,list):
  assert isinstance(actual,list)
  assert len(actual)==len(expected)
  return [project_legacy(a,b) for a,b in zip(actual,expected)]
 assert actual==expected,(actual,expected)
 return actual
