namespace Umbra
{
namespace License
{

#ifdef UMBRA_UNLOCKED
extern const int g_requireValidation = 0;
#else
extern const int g_requireValidation = 1;
#endif

}
}
