#ifndef _UMBRACMDLINEAPP_HPP
#define _UMBRACMDLINEAPP_HPP

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
 * \brief   Simple command line app utility
 *
 */

#include "umbraPrivateDefs.hpp"
#include "umbraString.hpp"
#include "umbraArray.hpp"
#include "umbraFloat.hpp"

namespace Umbra
{

class CmdlineLogger : public Logger
{
public:
    CmdlineLogger(bool& quiet) : m_quiet(quiet) {}
    void log (Level, const char* str);

private:
    CmdlineLogger& operator=(const CmdlineLogger&);
    bool& m_quiet;
};

class CmdlineApp
{
private:
    class Option
    {
    public:
                                        Option                  (void);
                                        Option                  (const String& name,
                                                                 const String& alterName,
                                                                 const String& valueName,
                                                                 const String& description);
                                        Option                  (const Option& o);
                                        ~Option                 (void);

        Option&                         operator=               (const Option& o);

        UMBRA_FORCE_INLINE const String&    getName                 (void) const        { return m_name; }
        UMBRA_FORCE_INLINE const String&    getAlterName            (void) const        { return m_alterName; }
        UMBRA_FORCE_INLINE const String&    getValueName            (void) const        { return m_valueName; }
        UMBRA_FORCE_INLINE const String&    getDescription          (void) const        { return m_description; }
        UMBRA_FORCE_INLINE bool         needsValue              (void) const        { return (m_valueName.length() != 0); }

        UMBRA_FORCE_INLINE bool         matches                 (const String& name) const { return (name == m_name || (m_alterName.length() && name == m_alterName)); }
        UMBRA_FORCE_INLINE bool         matches                 (const Option& o) const { return (o.matches(m_name) || (m_alterName.length() && o.matches(m_alterName))); }
        UMBRA_FORCE_INLINE void         addValue                (const String& value) { m_values.pushBack(value); }
        UMBRA_FORCE_INLINE bool         hasValues               (void) const        { return (m_valueIndex < m_values.getSize()); }
        UMBRA_FORCE_INLINE const String&    getValue                (void)              { UMBRA_ASSERT(hasValues()); return m_values[m_valueIndex++]; }

    private:
        String                          m_name;
        String                          m_alterName;
        String                          m_valueName;
        String                          m_description;
        Array<String>                   m_values;
        int32                           m_valueIndex;
    };

public:
        enum LogLevel
        {
            LogLevel_Info,
            LogLevel_Warning,
            LogLevel_Error
        };

        CmdlineApp(const String& name);
        virtual ~CmdlineApp (void);

        int execute (int argc = 0, char* argv[] = NULL);
        int execute (const char* commandLine);

protected:

        virtual void                    displayCustomHelp       (void);
        virtual void                    init                    (void) = 0;
        virtual void                    run                     (void) = 0;

        void                            declareOption           (const String& name,
                                                                 const String& alterName,
                                                                 const String& valueName,
                                                                 const String& description);
        UMBRA_FORCE_INLINE bool         hasOption               (const String& name) const { return (m_options[findOption(name)].hasValues()); }
        bool                            getBoolOption           (const String& name);
        UMBRA_FORCE_INLINE const String&    getStringOption         (const String& name) { return m_options[findOption(name)].getValue(); }
        Float                           getFloatOption          (const String& name);
        int                             getIntOption            (const String& name);

        UMBRA_FORCE_INLINE const String& getName                 (void) const { return m_name; }
        UMBRA_FORCE_INLINE const String& getCommandLine          (void) const { return m_commandLine; }
        UMBRA_FORCE_INLINE const String& getExecutable           (void) const { return m_executable; }
        UMBRA_FORCE_INLINE bool          isQuiet                 (void) const { return m_quiet; }
        UMBRA_FORCE_INLINE bool          isAborted               (void) const { return (m_errorcode != 0); }

        void                            error                   (UINT32 code, const char* message, ...);
        void                            log                     (LogLevel level, const char* message, ...);
        Logger*                         getLogger               (void) { return (Logger*)m_logger; }

private:

        int                             findOption              (const String& name) const;
        void                            parseCmdline            (const char* cmdline);
        bool                            hasMoreOptions          (void);
        void                            displayHelp             (void);

        String                          m_name;
        UINT32                          m_errorcode;

        Array<Option>                   m_options;
        String                          m_executable;
        String                          m_commandLine;
        bool                            m_quiet;
        CmdlineLogger*                  m_logger;
};

} // namespace Umbra

#endif //_UMBRACMDLINEAPP_HPP
