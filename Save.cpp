#include "Save.h"
#include <cstdio>

void Save::SaveFile()
{
	FILE* f;
#if _WIN32
	fopen_s(&f, "save.sav", "w");
#else
	f = fopen("save.sav", "w");
#endif
	if (f == NULL)
		return;

	fprintf(f, "%s", password.c_str());
	fprintf(f, "\n%d", key_idx);
	fprintf(f, "\n%d", port);
	fprintf(f, "\n%d", parameter_multiplexing);
	fprintf(f, "\n%d", refresh_rate);
	fprintf(f, "\n%d", start_and_hide);
	fclose(f);
}
