#include "services/fuzzy.hpp"
#include <stdexcept>
using namespace meridian::search;
void require(bool ok) { if (!ok) throw std::runtime_error("fuzzy contract failed"); }
std::optional<int> score(const char *query, const char *text) { return fuzzy(tokens(query).at(0), Prepared(text)); }
int main() {
    require(score("frf", "Firefox").has_value());
    require(!score("xf", "Firefox"));
    require(score("FIRE", "firefox") == score("fire", "Firefox"));
    require(score("term", "Terminal") > score("term", "Xterm Emulator"));
    require(score("code", "Visual Studio Code") > score("code", "Barcode Scanner"));
    require(score("fire", "Firefox") > score("fire", "File Reader"));
    require(score("lo", "LibreOffice") > score("lo", "Hello World"));
    require(score("lo", "Lorem") > score("lo", "LibreOffice"));
    require(score("é", "Éditeur de texte").has_value());
    require(Prepared("İstanbul").chars.size() == 8);
    Fields app{Prepared("Firefox"), Prepared("Browse the World Wide Web"), {Prepared("Web Browser"), Prepared("Internet")}};
    require(app.score(tokens("ffx")).has_value());
    require(app.score(tokens("web fire")).has_value());
    require(app.score(tokens("world")).has_value());
    require(!app.score(tokens("orld")));
    require(!app.score(tokens("firefox terminal")));
    require(app.score(tokens("  \t")).value() == 0);
}
