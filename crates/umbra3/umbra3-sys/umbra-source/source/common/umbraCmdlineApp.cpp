/*!
 *
 * Umbra common
 * -----------------------------------------
 *
 * (C) 2010 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Command line option parser
 *
 */

#include "umbraPrivateDefs.hpp"
#if UMBRA_ARCH != UMBRA_SPU

#include "umbraCmdlineApp.hpp"
#include "umbraFloat.hpp"
#include <cstdarg>

#if UMBRA_OS == UMBRA_XBOX360
#define NOD3D
#define NONET
#   include <xtl.h>
#elif UMBRA_OS == UMBRA_XBOXONE
#   include <Windows.h>
#endif

#define DEFAULT_EXE_NAME "umbravalidator"

using namespace std;

namespace Umbra
{

static void vlog(CmdlineApp::LogLevel level, const char* message, va_list args)
{
    FILE* out;

    static const char* const levelString[] =
    {
        "INFO", "WARNING", "ERROR"
    };

    UMBRA_ASSERT(level >= 0 && level < (int)UMBRA_ARRAY_SIZE(levelString));

    if (level >= CmdlineApp::LogLevel_Error)
        out = stderr;
    else
        out = stdout;

    fprintf(out, "%s: ", levelString[level]);
    vfprintf(out, message, args);

#if UMBRA_OS == UMBRA_XBOXONE
    char temp[512] = "";
    vsnprintf_s<512>(temp, 512 - 1, message, args);
    OutputDebugStringA(levelString[level]);
    OutputDebugStringA(": ");
    OutputDebugStringA(temp);
    OutputDebugStringA("\n");
#endif

    if (message[strlen(message) - 1] != '\n')
        fprintf(out, "\n");

    fflush(out);
}

static void vlog(CmdlineApp::LogLevel level, const char* message, ...)
{
    va_list va;
    va_start(va, message);
    vlog(level, message, va);
    va_end(va);
}

void CmdlineLogger::log (Level level, const char* str)
{
    if (m_quiet)
        return;

    CmdlineApp::LogLevel cmdLevel = CmdlineApp::LogLevel_Info;
    switch(level)
    {
    case Logger::LEVEL_ERROR:
        cmdLevel = CmdlineApp::LogLevel_Error;
        break;
    case Logger::LEVEL_WARNING:
        cmdLevel = CmdlineApp::LogLevel_Warning;
        break;
    default:
        break;
    }

    vlog(cmdLevel, str);    
}



CmdlineApp::Option::Option(void)
{
    // empty
}

CmdlineApp::Option::Option(const String& name,
                           const String& alterName,
                           const String& valueName,
                           const String& description)
:   m_name          (name),
    m_alterName     (alterName),
    m_valueName     (valueName),
    m_description   (description),
    m_valueIndex    (0)
{
    // empty
}

CmdlineApp::Option::Option(const Option& o)
{
    operator=(o);
}

CmdlineApp::Option::~Option(void)
{
    // empty
}

CmdlineApp::Option& CmdlineApp::Option::operator=(const Option& o)
{
    if (&o != this)
    {
        m_name          = o.m_name;
        m_alterName     = o.m_alterName;
        m_valueName     = o.m_valueName;
        m_description   = o.m_description;
        m_values        = o.m_values;
        m_valueIndex    = o.m_valueIndex;
    }
    return *this;
}

CmdlineApp::CmdlineApp(const String& name)
{
    m_name = name;
    m_errorcode = 0;
    m_quiet = false;

    declareOption("-h", "--help", "", "Displays the list of supported command options and exits.");
    declareOption("-q", "--quiet", "", "Disables the output of all messages.");

    m_logger = UMBRA_NEW(CmdlineLogger, m_quiet);
}

CmdlineApp::~CmdlineApp (void)
{
    UMBRA_DELETE(m_logger);
}

int CmdlineApp::execute(int argc, char *argv[])
{
    char* cmdLine = NULL;
    int ret;

    if (argc > 0)
    {
        int length = 0;
        if (strlen(argv[0]) == 0)
            length += (int)strlen(DEFAULT_EXE_NAME);
        for (int i = 0; i < argc; i++)
            length += (int)strlen(argv[i])+1;
        cmdLine = UMBRA_NEW_ARRAY(char, length);
        char* dst = cmdLine;
        for (int i = 0; i < argc; i++)
        {
            int cmdLength = (int)strlen(argv[i]);
            if (i == 0 && cmdLength == 0)
            {
                cmdLength = (int)strlen(DEFAULT_EXE_NAME);
                memcpy(dst, DEFAULT_EXE_NAME, cmdLength);
            }
            else
            {
                memcpy(dst, argv[i], cmdLength*sizeof(char));
            }
            dst[cmdLength] = ' ';
            dst += cmdLength+1;
        }
        cmdLine[length-1] = 0;
    }

#if UMBRA_OS == UMBRA_XBOX360
    ret = execute(GetCommandLine());
#else
    ret = execute(cmdLine);
#endif

    UMBRA_DELETE_ARRAY(cmdLine);
    return ret;
}

int CmdlineApp::execute(const char* cmdline)
{
    // parse command line into options

    parseCmdline(cmdline);

    if (isAborted())
        return m_errorcode;

    // process options

    if (getBoolOption("-h"))
    {
        displayHelp();
        return 0;
    }
    if (getBoolOption("-q"))
    {
        m_quiet = true;
    }

    // run app init

    init();

    if (isAborted())
        return m_errorcode;

    UMBRA_ASSERT(!hasMoreOptions() || !"Unprocessed options declared!");

    // enter app main routine

    run();

    return m_errorcode;
}

void CmdlineApp::log(LogLevel level, const char* message, ...)
{
    if (m_quiet)
        return;
    va_list va;
    va_start(va, message);
    vlog(level, message, va);
    va_end(va);
}

void CmdlineApp::error(Umbra::UINT32 code, const char* message, ...)
{
    UMBRA_ASSERT(code != 0);

    if (m_errorcode)
        return;
    m_errorcode = code;

    va_list va;
    va_start(va, message);
    vlog(LogLevel_Error, message, va);
    va_end(va);
}

void CmdlineApp::declareOption(const String& name,
                                 const String& alterName,
                                 const String& valueName,
                                 const String& description)
{
    Option o(name, alterName, valueName, description);
#ifdef UMBRA_DEBUG
    for (int i = 0; i < m_options.getSize(); i++)
        UMBRA_ASSERT(!m_options[i].matches(o));
#endif
    m_options.pushBack(o);
}

int CmdlineApp::findOption(const String& name) const
{
    int idx = -1;
    for (int i = 0; i < m_options.getSize(); i++)
        if (m_options[i].matches(name))
        {
            idx = i;
            break;
        }
    UMBRA_ASSERT(idx != -1);
    return idx;
}

bool CmdlineApp::getBoolOption(const String& name)
{
    int idx = findOption(name);
    if (!m_options[idx].hasValues())
        return false;
    m_options[idx].getValue();
    return true;
}

Float CmdlineApp::getFloatOption(const String& name)
{
    String sval = m_options[findOption(name)].getValue();
    float fval;
    int dummy;
    if (sscanf((sval + "\n0").toCharPtr(), "%f\n%d", &fval, &dummy) != 2)
    {
        error(1007, "Float value expected '%s'.", sval.toCharPtr());
        return 0.0f;
    }
    else if (!Float(fval).isFinite())
    {
        error(1007, "Float value out of range '%s'.", sval.toCharPtr());
        return 0.0f;
    }
    return fval;
}

int CmdlineApp::getIntOption(const String& name)
{
    String sval = m_options[findOption(name)].getValue();
    int ival;
    int dummy;
    if (sscanf((sval + "\n0").toCharPtr(), "%d\n%d", &ival, &dummy) == 2)
        return ival;
    error(1007, "Integer value expected '%s'.", sval.toCharPtr());
    return 0;
}

void CmdlineApp::parseCmdline(const char* rawCommandLine)
{
    Array<String> tokens;
    String cmdLine(rawCommandLine);
    String token;
    bool quoted = false;

    // tokenize

    for (int i = 0; i < cmdLine.length(); i++)
    {
        char c = cmdLine[i];
        if (c == '\"')
        {
            quoted = !quoted;
        }
        else if (quoted || (c != ' ' && c != '\t'))
        {
            token += c;
        }
        else if (token.length())
        {
            tokens.pushBack(token);
            token = "";
        }
    }

    if (token.length())
        tokens.pushBack(token);

    // reform command-line

    m_commandLine = "";
    for (int i = 1; i < tokens.getSize(); i++)
    {
        bool hasSpaces = false;
        for (int j = 0; j < tokens[i].length(); j++)
            if (tokens[i][j] == ' ' || tokens[i][j] == '\t')
            {
                hasSpaces = true;
                break;
            }
        if (i != 1)
            m_commandLine += ' ';
        if (hasSpaces)
            m_commandLine += '\"';
        m_commandLine += tokens[i];
        if (hasSpaces)
            m_commandLine += '\"';
    }

    // get options

    for (int i = 1; i < tokens.getSize(); i++)
    {
        bool match = false;
        for (int j = 0; j < m_options.getSize(); j++)
        {
            if (m_options[j].matches(tokens[i]))
            {
                if (m_options[j].needsValue())
                {
                    if (i == tokens.getSize() - 1)
                    {
                        error(1005, "Option '%s' requires a parameter.",
                            tokens[i].toCharPtr());
                        return;
                    }
                    m_options[j].addValue(tokens[++i]);
                }
                else
                {
                    m_options[j].addValue("");
                }
                match = true;
                break;
            }
        }
        if (!match)
        {
            error(1006, "Unrecognized option '%s'.", tokens[i].toCharPtr());
            return;
        }
    }

    m_executable = tokens[0];
}

bool CmdlineApp::hasMoreOptions(void)
{
    for (int i = 0; i < m_options.getSize(); i++)
        if (m_options[i].hasValues())
            return true;
    return false;
}

void CmdlineApp::displayHelp(void)
{
    // title

    String titleString = m_name + " help";
    printf("\n%s\n", titleString.toCharPtr());
    for (int i = 0; i < titleString.length(); i++)
        printf("-");
    printf("\n\n");

    // custom help

    displayCustomHelp();

    // options

    Array<Array<String> > optionLines(m_options.getSize());
    for (int i = 0; i < m_options.getSize(); i++)
    {
        String valueString;
        if (m_options[i].needsValue())
        {
            valueString += ' ';
            valueString += m_options[i].getValueName();
        }
        if (m_options[i].getName().length())
            optionLines[i].pushBack(m_options[i].getName() + valueString);
        if (m_options[i].getAlterName().length())
            optionLines[i].pushBack(m_options[i].getAlterName() + valueString);
    }

    // calculate option tabulation

    int width = 78;
    int tab = 0;
    for (int i = 0; i < optionLines.getSize(); i++)
        for (int j = 0; j < optionLines[i].getSize(); j++)
            tab = max2(tab, optionLines[i][j].length() + 2);

    // print options

    for (int i = 0; i < m_options.getSize(); i++)
    {
        Array<String> descLines;
        m_options[i].getDescription().wordWrap(width - tab, descLines);
        for (int j = 0; j <= optionLines[i].getSize() || j <= descLines.getSize(); j++)
        {
            int pos = 0;
            if (j < optionLines[i].getSize() && optionLines[i][j].length())
            {
                printf("%s", optionLines[i][j].toCharPtr());
                pos += optionLines[i][j].length();
            }
            if (j < descLines.getSize() && descLines[j].length())
            {
                while (pos < tab)
                {
                    printf(" ");
                    pos++;
                }
                printf("%s", descLines[j].toCharPtr());
            }
            printf("\n");
        }
    }
}

void CmdlineApp::displayCustomHelp (void)
{
}

}

#endif // UMBRA_ARCH != UMBRA_SPU