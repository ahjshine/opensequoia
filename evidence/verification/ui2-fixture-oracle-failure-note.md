# Observed UI smoke oracle failure

This is a transcript note, **not an intact original raw log**. The original `ui2-app-smoke.log` was overwritten by a later successful run before the preserve request arrived. Tool session34707 reported:

```text
OpenSequoia: native smoke content mismatch at stage 0
Exception: app smoke failed
```

That command exited1 after root changed Welcome page copy to remove the word `rendering`, while the native smoke still required that word. Content verification was retained and corrected to current first-run identity: Welcome `OpenSequoia`, Guide `Browse`. Subsequent actual native smoke exited0 and is preserved in the current `ui2-app-smoke.log`; fixture/source hashes are separately recorded. No failing criterion was removed, and this note is not substituted for raw authenticated evidence.
